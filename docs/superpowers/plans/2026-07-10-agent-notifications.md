# Agent Notifications Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 统一记录并按窗口状态提醒 Claude Code、Codex、Gemini、Grok 和 OpenCode 的注意与完成事件。

**Architecture:** 五种 CLI 均发送命名 OSC 777 标记，由现有 Rust `AgentDetector` 和前端通知路由统一处理。Grok 复用 JSON Hook 合并器；OpenCode 使用一个 Terax 专用全局插件文件，不增加独立通知管线。

**Tech Stack:** Rust stable-msvc、Tauri 2、TypeScript 6、Zustand 5、Vitest 4、OpenCode JavaScript Plugin API。

## Global Constraints

- 只接受 `claude`、`codex`、`gemini`、`grok`、`opencode`。
- 只接受 `working`、`attention`、`finished`。
- 写用户配置前必须解析并保留原内容；无效内容不得覆盖。
- 不读取、输出或记录 API Key、Token 和完整 Agent 配置。
- 只有用户点击“启用”后才写 Agent 配置。
- 不新增依赖，不解析终端可见文本。

---

### Task 1: 通知记录和显示策略

**Files:**
- Create: `src/modules/agents/lib/route.test.ts`
- Modify: `src/modules/agents/lib/route.ts`
- Modify: `src/modules/agents/components/AgentNotificationsBridge.tsx`
- Modify: `src/modules/ai/components/LocalAgentNotificationsBridge.tsx`

**Interfaces:**
- Consumes: `RouteArgs` 中的 `focused`、`visible`、通知内容和激活回调。
- Produces: 所有有效事件先记录；失焦时系统通知；前台隐藏时应用内提示；前台可见时仅记录。

- [ ] **Step 1: 写入前台可见仍记录的失败测试**

创建 `route.test.ts`：

```ts
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
```

- [ ] **Step 2: 运行测试并确认 RED**

Run: `pnpm test src/modules/agents/lib/route.test.ts`

Expected: 两个测试均 FAIL；当前可见事件未记录，隐藏的 `finished` 被 `allowToast` 抑制。

- [ ] **Step 3: 实现最小通知顺序**

将 `routeAgentNotification` 的核心顺序改为：

```ts
if (!usePreferencesStore.getState().agentNotifications) return;

useAgentStore.getState().pushNotification({
  source,
  agent,
  kind,
  tabId,
  leafId,
});

if (focused && visible) return;
if (!focused) {
  void osNotify(title, body ?? agent);
  return;
}
showAgentToast({ agent, title, body, onActivate });
```

删除 `RouteArgs.allowToast`，并删除两个 Bridge 调用方传入的 `allowToast`。

- [ ] **Step 4: 运行测试并确认 GREEN**

Run:

```powershell
pnpm test src/modules/agents/lib/route.test.ts
pnpm check-types
```

Expected: 2 tests PASS，类型检查退出码为 0。

- [ ] **Step 5: 提交通知策略**

```powershell
git add src/modules/agents/lib/route.ts src/modules/agents/lib/route.test.ts src/modules/agents/components/AgentNotificationsBridge.tsx src/modules/ai/components/LocalAgentNotificationsBridge.tsx
git commit -m "fix(agents): 统一完成通知展示策略"
```

### Task 2: Rust 检测器接受 Grok 和 OpenCode

**Files:**
- Modify: `src-tauri/src/modules/pty/agent_detect.rs`

**Interfaces:**
- Produces: `AgentDetector::new()` 接受两个新增 Agent 的命名 OSC 标记和命令名。

- [ ] **Step 1: 写入新增 Agent 自启动的失败测试**

在现有测试模块加入：

```rust
#[test]
fn named_markers_self_arm_grok_and_opencode() {
    for agent in ["grok", "opencode"] {
        let mut detector = AgentDetector::new();
        assert_eq!(
            run(
                &mut detector,
                &osc(&format!("777;notify;Terax;{agent};finished")),
            ),
            vec![started(agent), Transition::Finished],
        );
    }
}
```

- [ ] **Step 2: 运行测试并确认 RED**

Run:

```powershell
Set-Location src-tauri
cargo test --locked named_markers_self_arm_grok_and_opencode
```

Expected: FAIL，实际转换为空，因为白名单尚未包含两个 Agent。

- [ ] **Step 3: 扩展唯一白名单**

```rust
const DEFAULT_AGENTS: &[&str] = &["claude", "codex", "gemini", "grok", "opencode"];
```

- [ ] **Step 4: 运行测试并确认 GREEN**

Run:

```powershell
cargo test --locked named_markers_self_arm_grok_and_opencode
cargo test --locked agent_detect
```

Expected: 新测试和现有检测器测试全部 PASS。

- [ ] **Step 5: 提交检测器白名单**

```powershell
git add src-tauri/src/modules/pty/agent_detect.rs
git commit -m "feat(agents): 识别 Grok 和 OpenCode"
```

### Task 3: Grok JSON Hook

**Files:**
- Modify: `src-tauri/src/modules/agent.rs`

**Interfaces:**
- Produces: `find("grok")` 返回指向 `.grok/hooks/terax.json` 的 `AgentSpec`。
- Consumes: 现有 `merge_hooks`、Windows `CONOUT$` 辅助入口和原子写入。

- [ ] **Step 1: 写入 Grok Hook 的失败测试**

```rust
#[test]
fn grok_adds_working_attention_and_finished_hooks() {
    let out = merge_hooks(json!({}), spec("grok"));
    assert_eq!(hook_count(&out, "UserPromptSubmit"), 1);
    assert_eq!(hook_count(&out, "Notification"), 1);
    assert_eq!(hook_count(&out, "Stop"), 1);
    assert!(command(&out, "Stop", 0).contains("__terax_notify grok finished"));
}
```

- [ ] **Step 2: 运行测试并确认 RED**

Run: `cargo test --locked grok_adds_working_attention_and_finished_hooks`

Expected: FAIL，`find("grok")` 返回 unknown agent。

- [ ] **Step 3: 添加 Grok AgentSpec**

在 `AGENTS` 中加入：

```rust
AgentSpec {
    agent: "grok",
    dir: ".grok/hooks",
    file: "terax.json",
    events: &[
        ("UserPromptSubmit", "working"),
        ("Notification", "attention"),
        ("Stop", "finished"),
    ],
    matcher: false,
    delivery: Delivery::Osc,
},
```

- [ ] **Step 4: 运行测试并确认 GREEN**

Run:

```powershell
cargo test --locked grok_adds_working_attention_and_finished_hooks
cargo test --locked modules::agent
```

Expected: Grok 和现有 Agent Hook 测试全部 PASS。

- [ ] **Step 5: 让 Windows 状态检查包含当前可执行文件**

先添加失败测试：

```rust
#[cfg(windows)]
#[test]
fn windows_status_needle_includes_current_executable() {
    let needle = status_needle(spec("codex"), "finished");
    let exe = std::env::current_exe().unwrap().display().to_string();
    assert!(needle.contains(&exe));
}
```

运行 `cargo test --locked windows_status_needle_includes_current_executable` 确认 RED，再让 Windows `status_needle` 返回完整 `hook_command(spec, event)`，重新运行确认 GREEN。

- [ ] **Step 6: 提交 Grok 和状态检查**

```powershell
git add src-tauri/src/modules/agent.rs
git commit -m "feat(agents): 添加 Grok Hook"
```

### Task 4: OpenCode 全局插件

**Files:**
- Modify: `src-tauri/src/modules/agent.rs`

**Interfaces:**
- Produces: `opencode_plugin_source(): &'static str`。
- Produces: `~/.config/opencode/plugins/terax-agent-notifications.js`。
- Produces: 专用文件所有权标记 `Managed by Terax: agent notifications`。

- [ ] **Step 1: 写入插件内容的失败测试**

```rust
#[test]
fn opencode_plugin_emits_attention_and_finished_markers() {
    let source = opencode_plugin_source();
    assert!(source.contains("Managed by Terax: agent notifications"));
    assert!(source.contains("permission.asked"));
    assert!(source.contains("opencode;attention"));
    assert!(source.contains("session.idle"));
    assert!(source.contains("opencode;finished"));
    assert!(source.contains("TERAX_TERMINAL"));
}
```

- [ ] **Step 2: 运行测试并确认 RED**

Run: `cargo test --locked opencode_plugin_emits_attention_and_finished_markers`

Expected: 编译失败，因为 `opencode_plugin_source` 尚不存在。这是该独立 TDD 循环的缺失接口证据；修复其他错误后不得跳过本次 RED。

- [ ] **Step 3: 添加最小插件源码**

```rust
fn opencode_plugin_source() -> &'static str {
    r#"// Managed by Terax: agent notifications
const emit = (event) => {
  if (process.env.TERAX_TERMINAL !== "1") return
  process.stdout.write(`\u001b]777;notify;Terax;opencode;${event}\u0007`)
}

export const TeraxAgentNotifications = async () => ({
  event: async ({ event }) => {
    if (event.type === "permission.asked") emit("attention")
    if (event.type === "session.idle") emit("finished")
  },
})
"#
}
```

- [ ] **Step 4: 运行源码测试并确认 GREEN**

Run: `cargo test --locked opencode_plugin_emits_attention_and_finished_markers`

Expected: PASS。

- [ ] **Step 5: 添加专用文件写入和状态分支**

`agent_enable_hooks("opencode")` 必须：

1. 解析路径 `~/.config/opencode/plugins/terax-agent-notifications.js`。
2. 文件不存在时创建父目录并原子写入插件源码。
3. 文件存在且包含所有权标记时更新。
4. 文件存在但没有所有权标记时返回错误，不覆盖。

`agent_hooks_status("opencode")` 仅在文件同时包含所有权标记、`session.idle` 和 `opencode;finished` 时返回 true。

- [ ] **Step 6: 用标准库临时目录测试所有权保护**

将文件判断提取为：

```rust
fn can_write_opencode_plugin(existing: Option<&str>) -> bool {
    existing.is_none_or(|text| text.contains("Managed by Terax: agent notifications"))
}
```

先写失败测试：

```rust
#[test]
fn opencode_plugin_refuses_foreign_file() {
    assert!(can_write_opencode_plugin(None));
    assert!(can_write_opencode_plugin(Some(opencode_plugin_source())));
    assert!(!can_write_opencode_plugin(Some("export const UserPlugin = () => ({})")));
}
```

运行确认 RED，再添加最小实现并确认 GREEN。

- [ ] **Step 7: 提交 OpenCode 插件**

```powershell
git add src-tauri/src/modules/agent.rs
git commit -m "feat(agents): 添加 OpenCode 完成通知插件"
```

### Task 5: 五种 Agent 的通知界面

**Files:**
- Modify: `src/modules/agents/components/NotificationBell.tsx`
- Modify: `src/modules/agents/lib/format.ts`
- Modify: `src/modules/agents/lib/agentIcon.tsx`

**Interfaces:**
- Consumes: `agent_enable_hooks`、`agent_hooks_status`。
- Produces: 五种 Agent 的启用入口和可见错误。

- [ ] **Step 1: 增加 Agent 列表和显示名称**

```ts
const HOOK_AGENTS = ["claude", "codex", "gemini", "grok", "opencode"] as const;
```

在 `LABELS` 中加入：

```ts
grok: "Grok",
opencode: "OpenCode",
```

继续使用现有通用机器人图标，不增加品牌资源。

- [ ] **Step 2: 安装失败时显示错误**

在 `enableHooks` 的 `catch` 中使用现有 Sonner：

```ts
} catch (error) {
  setHooks((current) => ({ ...current, [id]: false }));
  toast.error(`Failed to enable ${displayAgent(id)} hooks`, {
    description: String(error),
  });
}
```

i18n 任务完成后，这两段文本必须通过 `t()` 翻译。

- [ ] **Step 3: 运行完整验证**

Run:

```powershell
pnpm test src/modules/agents/lib/route.test.ts
pnpm check-types
pnpm lint
Set-Location src-tauri
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

Expected: 全部退出码为 0。

- [ ] **Step 4: Windows 人工验收**

分别启用五种 Agent，验证：

1. 当前可见完成时仅进入通知记录。
2. 前台其他标签完成时显示应用内提示。
3. Terax 失焦或最小化时显示 Windows 系统通知。
4. Hook 安装失败时显示错误且不覆盖用户配置。

- [ ] **Step 5: 提交通知界面**

```powershell
git add src/modules/agents/components/NotificationBell.tsx src/modules/agents/lib/format.ts src/modules/agents/lib/agentIcon.tsx
git commit -m "feat(agents): 展示五种 CLI 通知适配"
```
