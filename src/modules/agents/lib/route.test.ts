// 验证 Agent 通知先记录，再依据窗口焦点和可见性选择展示方式。
import { beforeEach, describe, expect, it, vi } from "vitest";

const deps = vi.hoisted(() => ({
  pushNotification: vi.fn(),
  showAgentToast: vi.fn(),
  osNotify: vi.fn(),
  preferences: { agentNotifications: true },
}));

vi.mock("@/modules/settings/preferences", () => ({
  usePreferencesStore: { getState: () => deps.preferences },
}));
vi.mock("../components/AgentToast", () => ({
  showAgentToast: deps.showAgentToast,
}));
vi.mock("../store/agentStore", () => ({
  useAgentStore: {
    getState: () => ({ pushNotification: deps.pushNotification }),
  },
}));
vi.mock("./notify", () => ({ osNotify: deps.osNotify }));

import { routeAgentNotification } from "./route";

function route(overrides: { focused: boolean; visible: boolean }) {
  routeAgentNotification({
    source: "terminal",
    agent: "codex",
    kind: "finished",
    title: "Codex finished",
    body: "workspace",
    allowToast: false,
    tabId: 1,
    leafId: 2,
    onActivate: vi.fn(),
    ...overrides,
  });
}

describe("routeAgentNotification", () => {
  beforeEach(() => vi.clearAllMocks());

  it("records without popup when the completed agent is visible", () => {
    route({ focused: true, visible: true });
    expect(deps.pushNotification).toHaveBeenCalledOnce();
    expect(deps.showAgentToast).not.toHaveBeenCalled();
    expect(deps.osNotify).not.toHaveBeenCalled();
  });

  it("shows an in-app toast when the completed agent is hidden", () => {
    route({ focused: true, visible: false });
    expect(deps.pushNotification).toHaveBeenCalledOnce();
    expect(deps.showAgentToast).toHaveBeenCalledOnce();
    expect(deps.osNotify).not.toHaveBeenCalled();
  });
});
