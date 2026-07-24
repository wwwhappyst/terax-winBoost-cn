import { usePreferencesStore } from "@/modules/settings/preferences";
import { showAgentToast } from "../components/AgentToast";
import { useAgentStore } from "../store/agentStore";
import { osNotify } from "./notify";
import { setPendingAgentActivate } from "./pendingActivate";
import type { AgentSource, NotificationKind } from "./types";

type RouteArgs = {
  source: AgentSource;
  agent: string;
  kind: NotificationKind;
  title: string;
  body?: string;
  focused: boolean;
  /** True when the user is currently looking at this agent. */
  visible: boolean;
  tabId?: number;
  leafId?: number;
  onActivate: () => void;
};

export function routeAgentNotification({
  source,
  agent,
  kind,
  title,
  body,
  focused,
  visible,
  tabId = 0,
  leafId = 0,
  onActivate,
}: RouteArgs): void {
  if (!usePreferencesStore.getState().agentNotifications) return;

  // 所有有效事件先进入通知记录，再按用户当前视线决定展示方式。
  useAgentStore
    .getState()
    .pushNotification({ source, agent, kind, tabId, leafId });

  if (focused && visible) return;
  if (!focused) {
    // Windows 通知点击只会把窗口拉到前台，不会回调 onActivate；
    // 记下待办，等焦点回来时切到对应页签并做输入恢复。
    setPendingAgentActivate(onActivate);
    void osNotify(title, body ?? agent);
    return;
  }
  showAgentToast({ agent, title, body, onActivate });
}
