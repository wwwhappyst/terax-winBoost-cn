import type { Tab } from "@/modules/tabs";
import { t } from "@/modules/i18n";
import { hasLeaf, leafIdForPty } from "@/modules/terminal";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import { displayAgent } from "../lib/format";
import { maybeTriggerManagedReview } from "../lib/review";
import { routeAgentNotification } from "../lib/route";
import type { AgentSession, AgentSignal } from "../lib/types";
import { useWindowFocus } from "../lib/useWindowFocus";
import { useAgentStore } from "../store/agentStore";
import { useManagedAgentsStore } from "../store/managedAgentsStore";

type Activate = (tabId: number, leafId: number) => void;
type Ctx = {
  tabs: Tab[];
  activeId: number;
  focused: boolean;
  onActivate: Activate;
};

/** 同一 leaf 在 finished 后短时间内忽略 attention，避免 Stop+Notification 连弹。 */
const ATTENTION_SUPPRESS_MS = 3000;
const recentFinishedAt = new Map<number, number>();

function tabInfo(
  tabs: Tab[],
  leafId: number,
): { tabId: number; title: string } | null {
  for (const t of tabs) {
    if (t.kind === "terminal" && hasLeaf(t.paneTree, leafId)) {
      return { tabId: t.id, title: t.title };
    }
  }
  return null;
}

function route(
  session: AgentSession,
  kind: "attention" | "finished",
  ctx: Ctx,
): void {
  const info = tabInfo(ctx.tabs, session.leafId);
  const name = displayAgent(session.agent);
  const heading =
    kind === "attention"
      ? t("{agent} needs your input", { agent: name })
      : t("{agent} finished", { agent: name });

  routeAgentNotification({
    source: "terminal",
    agent: session.agent,
    kind,
    title: heading,
    body: info?.title,
    focused: ctx.focused,
    visible: ctx.activeId === session.tabId,
    tabId: session.tabId,
    leafId: session.leafId,
    onActivate: () => ctx.onActivate(session.tabId, session.leafId),
  });
}

function handleSignal(sig: AgentSignal, ctx: Ctx): void {
  const leafId = leafIdForPty(sig.id);
  if (leafId === null) return;
  const store = useAgentStore.getState();

  switch (sig.kind) {
    case "started": {
      const info = tabInfo(ctx.tabs, leafId);
      if (!info) return;
      const agent = sig.agent ?? "agent";
      // Grok 会误跑 ~/.claude hooks：已识别为其它 agent 时忽略 claude 串台。
      const existing = store.sessions[leafId];
      if (
        existing &&
        agent === "claude" &&
        existing.agent !== "claude"
      ) {
        return;
      }
      store.start(leafId, info.tabId, agent);
      return;
    }
    case "working":
      store.setStatus(leafId, "working");
      return;
    case "attention": {
      const finishedAt = recentFinishedAt.get(leafId);
      if (
        finishedAt !== undefined &&
        Date.now() - finishedAt < ATTENTION_SUPPRESS_MS
      ) {
        return;
      }
      store.setStatus(leafId, "waiting");
      const session = store.sessions[leafId];
      if (session) route(session, "attention", ctx);
      return;
    }
    case "finished": {
      // 回合结束：写入「已完成」通知，并移出活动任务。
      // 勿标成 waiting，否则铃铛会显示「等待中」，与「需要输入」混淆。
      recentFinishedAt.set(leafId, Date.now());
      const session = store.sessions[leafId];
      if (session) {
        route(session, "finished", ctx);
        store.finish(leafId);
        store.startPulse(leafId, session.tabId);
      } else {
        store.finish(leafId);
      }
      maybeTriggerManagedReview(leafId);
      return;
    }
    case "exited":
      store.finish(leafId);
      store.clearPulse(leafId);
      useManagedAgentsStore.getState().remove(leafId);
      return;
  }
}

export function AgentNotificationsBridge({
  tabs,
  activeId,
  onActivate,
}: {
  tabs: Tab[];
  activeId: number;
  onActivate: Activate;
}) {
  const focused = useWindowFocus();
  const ctxRef = useRef<Ctx>({ tabs, activeId, focused, onActivate });
  ctxRef.current = { tabs, activeId, focused, onActivate };

  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    listen<AgentSignal>("terax:agent-signal", (e) =>
      handleSignal(e.payload, ctxRef.current),
    )
      .then((u) => {
        if (alive) unlisten = u;
        else u();
      })
      .catch(() => {});
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  return null;
}
