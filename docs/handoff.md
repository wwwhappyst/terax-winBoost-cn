# Terax Windows 改造接力

## 当前状态

- 工作分支：`codex/windows-enhancements`。
- 远程：当前仅有个人 Fork `origin`（`https://github.com/wwwhappyst/terax-ai.git`）；尚未配置原项目 `upstream`。
- 本地已完成本轮规格中的三项 Windows 改造及个人构建保护，未推送远程、未打包发布。
- 长期规格见 `docs/spec.md`，执行命令见 `docs/exec_mode.md`。

## 已完成内容

1. Windows 终端快捷键：有选区时 `Ctrl+C` 复制；无选区时保留发送中断；`Ctrl+V` 粘贴。实现位于 `src/modules/terminal/*`，仅 Windows 分支生效。
2. 界面汉化：偏好设置新增 `en` / `zh-CN`，常规可见界面文本通过 `src/modules/i18n/` 与共享 UI 组件翻译；终端输出、模型名、命令、开发日志和底层错误保持英文。设置窗口的原生标题会随语言同步。
3. Agent 完成通知：Claude Code、Codex、Gemini、Grok（命令为 `grok`）和 OpenCode 均接入完成/注意事件；通知策略为最小化或失焦时 Windows 系统通知、前台其他标签页应用内提示、当前标签仅记录。
4. 个人构建保护：移除了应用入口中的更新弹窗与“检查更新”入口，避免个人构建调用原作者更新流程。未来仅在具备个人签名密钥和个人 Release 源后再恢复。
5. 测试基础设施：`scripts/eager-graph.mjs` 删除 Node shebang。Vite 测试运行时会在模块顶部注入 import，shebang 留在中间会导致语法错误；删除后 eager-budget 测试恢复。

## 关键文件

- `src/modules/i18n/index.ts`、`src/modules/i18n/zh-CN.ts`：翻译函数与中文词典。
- `src/modules/i18n/index.test.ts`、`src/modules/i18n/react.test.tsx`：翻译核心的 RED/Green 样例。
- `src/settings/SettingsApp.tsx`：设置窗口语言及原生标题同步。
- `src/app/App.tsx`、`src/settings/sections/AboutSection.tsx`：个人构建中禁用更新入口。
- `src/modules/agents/`、`src/modules/ai/components/*Notifications*`：CLI Hook 与通知展示。
- `src-tauri/src/modules/agent.rs`、`src-tauri/src/modules/pty/agent_detect.rs`：Windows CLI Hook 和 PTY 标记检测。

## 已核验命令

```powershell
pnpm check-types
pnpm test
pnpm lint
$env:CARGO_HOME='D:\dev_tools\cargo'; $env:RUSTUP_HOME='D:\dev_tools\rustup'; cargo test --locked
node scripts/eager-graph.mjs src/main.tsx "@ai-sdk,ai,streamdown,@codemirror,@uiw"
```

- `pnpm test`：45 个文件、312 项通过。
- `cargo test --locked`：191 个 Rust 单元测试、25 个文件系统集成测试、27 个 Git 集成测试通过。
- `pnpm lint`：命令成功；保留仓库既有的 Biome 告警，本轮未扩大范围处理。
- eager 图检查未发现被观察的重型包进入启动 eager 图。

## 仍需人工验收

在 Windows 实机运行 `pnpm tauri dev` 后确认：

1. pwsh 中有选区/无选区的 `Ctrl+C` 与 `Ctrl+V` 行为。
2. 设置中切换简体中文后，主窗口、设置窗口、弹窗、菜单和通知文案刷新；终端及技术错误仍为英文。
3. 分别运行 `gemini`、`codex`、`claude`、`grok`、`opencode`，验证三种焦点状态的通知策略。

## 同步上游

首次配置上游后再同步，任何冲突均人工处理：

```powershell
git remote add upstream https://github.com/crynta/terax-ai.git
git fetch upstream
git switch main
git merge --ff-only upstream/main
git switch codex/windows-enhancements
git rebase main
```

不要把本分支直接推到原作者仓库；推送个人 `origin` 前仍需单独确认。
