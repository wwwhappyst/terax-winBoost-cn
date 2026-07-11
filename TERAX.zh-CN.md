# TERAX.md（中文译本）

> 本文档为 [TERAX.md](TERAX.md) 的中文译本，仅供阅读参考。以英文原文为准。

Terax 从工作区根目录加载 `TERAX.md` 作为 Agent 记忆（类似 AGENTS.md / CLAUDE.md）。本文件也是项目的活架构文档，修改代码前请先阅读。

## 项目

**Terax**：开源 AI 原生终端模拟器。Tauri 2 + Rust（`portable-pty`）后端，React 19 + TypeScript + xterm.js（webgl）客户端，通过 Vercel AI SDK v6 实现 BYOK AI。

- Bundle id：`app.crynta.terax`
- 包管理器：**pnpm**
- 平台：macOS、Linux、Windows
- 前端检查：`pnpm lint`、`pnpm check-types`、`pnpm test`
- Rust 检查：`cd src-tauri && cargo clippy --all-targets --locked -- -D warnings`、`cd src-tauri && cargo nextest run --locked`（本地回退：`cargo test --locked`）

## 质量门槛

达不到生产级就不发布。每项改动都按以下全部标准评判，而非仅「能跑就行」：

- **正确性**：边界情况、失败模式、并发访问。不接受「暂时能用」。
- **性能**：超轻量是产品定位。约 7-8 MB 安装包、高性能终端。每次改动都要问：占用多少 RAM、是否增加 IPC 往返或冗余请求、是否触发额外重渲染或无效工作、是否引入沉重依赖。未使用的功能应零资源消耗。
- **安全**：无严重安全漏洞。在每个边界（IPC、文件系统、网络、AI 工具面）做校验。密钥路径 deny-list 在读写两侧均生效，且不得绕过。
- **UI/UX**：精致、专业、高级感。每个状态与细节都要考虑。
- **架构**：新增或变更的逻辑放在纯函数、轻依赖的函数式核心中；Tauri 命令与 React 组件保持薄壳（命令式外壳）。便于测试，避免日后重写。

声称完成前须验证：

- 前端：`pnpm lint`、`pnpm check-types`、`pnpm test`
- Rust：`cd src-tauri && cargo clippy --all-targets --locked -- -D warnings`、`cd src-tauri && cargo nextest run --locked`（或 `cargo test --locked`）

涉及核心子系统（终端/Shell 启动、工作区授权、git、文件系统、IPC 或 AI 工具面）的改动，须有测试锁定不变量。

## 约定

- **注释**：默认不写，代码应自解释。确有需要时 1-2 行说明*为何*，而非*做什么*。禁止 AI 套话填充。
- **全文禁止 em-dash**：代码、注释、提交、文档。
- **全文禁止 emoji**。
- **导入**：前端始终用 `@/...`，模块间不用相对路径。
- **仅 pnpm**，不用 npm/npx/yarn。

## 架构

### 双进程模型

**Rust（`src-tauri/`）** 拥有全部 OS 访问权。Webview 不直接碰文件系统、进程或 Shell，一切通过 `invoke()` 调用在 `src-tauri/src/lib.rs` 注册的命令：

- `pty::pty_*`：长生命周期交互式 PTY 会话（xterm ↔ portable-pty），由 `PtyState`（`RwLock<HashMap<id, Session>>`）管理。输出经 Tauri `Channel<PtyEvent>` 流式推送。
- `fs::tree::*`（`fs_read_dir`、`list_subdirs`）、`fs::file::*`（`fs_read_file`、`fs_write_file`、`fs_stat`、`fs_canonicalize`）、`fs::mutate::*`（`fs_create_file`、`fs_create_dir`、`fs_rename`、`fs_delete`）：文件浏览器 + 编辑器 IO。
- `fs::search::*`（`fs_search`、`fs_list_files`）、`fs::grep::*`（`fs_grep`、`fs_glob`）：模糊文件查找 + 内容搜索（`ignore` + `grep-*` crates）。
- `git::commands::*`：完整源码控制面（`git_status`、`git_diff`、`git_diff_content`、`git_stage`、`git_unstage`、`git_discard`、`git_commit`、`git_fetch`、`git_pull_ff_only`、`git_push`、`git_log`、`git_show_commit`、`git_commit_files`、`git_commit_file_diff`、`git_panel_snapshot`、`git_resolve_repo`、`git_remote_url`）。均经工作区授权注册表门禁。
- `shell::shell_run_command`：AI 工具用的一次性子 Shell 执行。与 PTY 会话不同；不是用户交互终端。Windows 经 PowerShell（`-NoProfile -Command`），Unix 经 `$SHELL -lc`。共享辅助函数 `build_oneshot_command`。
- `shell::shell_session_*`：跨调用保持状态的持久 Agent Shell。`shell::shell_bg_*`（`spawn`、`logs`、`kill`、`list`）：长驻后台进程（开发服务器等），有界环形缓冲区日志采集。
- `workspace::*`：`workspace_authorize` / `workspace_current_dir`（启动/git/AI 的 cwd 授权注册表）及 WSL 桥（`wsl_list_distros`、`wsl_default_distro`、`wsl_home`）。
- `lsp::*`（`lsp_detect`、`lsp_host_pid`、`lsp_resolve_root`、`lsp_spawn`、`lsp_send`、`lsp_kill`）：语言服务器进程宿主。哑 JSON-RPC 管道：Rust 侧 Content-Length 分帧 + 进程生命周期（`lsp/framing.rs`，纯函数且已测试），协议智能在前端。启动 cwd 经工作区注册表门禁；二进制经捕获的登录 Shell 环境解析（`lsp/env.rs`，macOS GUI 应用 PATH 很 bare）；根目录检测向上找标记，但不到或超过 `$HOME`。Unix 上服务器在独立进程组，组杀（cargo check / proc-macro 子进程随服务器消亡）；Windows 子进程挂 `proc::job::ProcessJob`（关闭即杀，与 pty 共享）。`RunEvent::Exit` 时杀掉所有会话。
- `net::*`（`ai_http_request`、`ai_http_stream`、`lm_ping`）：带 SSRF 防护的 AI HTTP 代理；提供商调用与本地模型 ping 不经过 webview。
- `secrets::secrets_*`：经 `keyring` crate 访问 OS 钥匙串。服务常量 `terax-ai`。Linux 用文件回退，门控在 `#[cfg(target_os = "linux")]`。
- `open_settings_window`：设置独立 webview 窗口（可选 `tab` 参数深链到某节）。

### PTY Shell 集成

PTY Shell 通过 `src-tauri/src/modules/pty/scripts/` 注入的初始化脚本引导：

- **Unix**（`zshenv.zsh`、`zprofile.zsh`、`zlogin.zsh`、`zshrc.zsh`、`bashrc.bash`）用于 zsh/bash；fish 用 `init.fish` 装到 `~/.config/fish/conf.d/terax.fish`。发出 OSC 7（cwd）和 OSC 133 A/B/C/D（提示符边界 + 退出码），宿主可跟踪 cwd 并检测命令边界而无需重解析提示符。Fish 4.0+ 自带 OSC 133；Terax 设 `fish_features=no-mark-prompt` 并用 `-C` 重设自己的 prompt，避免重复。
- **Windows**（`profile.ps1`）：经 `pwsh -NoLogo -NoExit -ExecutionPolicy Bypass -File <path>` 传入。在用户 `$PROFILE` 运行后包装现有 `prompt`，发出 OSC 7 + OSC 133 A/B/D。Shell 优先级：`pwsh.exe`（PS 7+）→ `powershell.exe`（PS 5.1）→ `cmd.exe`（无集成）。传给 ConPTY 前 cwd 规范为反斜杠（`CreateProcessW` 对正斜杠 cwd 行为异常）。

`pty/shell_init.rs` 拆成 `#[cfg(unix)]` / `#[cfg(windows)]` 模块，新平台相关代码放在对应 cfg 分支。

Windows ConPTY 在 `session.rs` 的 `openpty + spawn_command` 外需要 `SPAWN_LOCK`（Mutex）。并发 spawn 会导致其中一个 PTY 输出管道卡住。未经快速开标签验证首标签稳定性，不要移除该锁。

每个 ConPTY 子进程还挂到每会话 **Job Object**，`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`（`pty/job.rs`）。Job HANDLE 释放时（正常退出、panic、甚至 Terax 被 SIGKILL），内核会杀掉 Shell 全部后代（如 pwsh 内 `npm run dev`）。否则 Windows 会孤儿整棵子树，因 `TerminateProcess` 只杀直接子进程。macOS/Linux 靠 `Drop for Session → killer.kill()`；开发时 `cargo run` 的 Ctrl-C 析构可能不跑，也可能孤儿，目前开发场景可接受。

`AiComposerProvider` 在 App.tsx 根无条件挂载：条件包装会在密钥加载时改变父元素类型，整树重挂载（并重开所有 PTY）。生产环境碰巧躲过（钥匙串读取可能同帧完成）；开发环境没有。保持无条件包装。

### 前端（`src/`）

单窗口 React 应用。路径别名 `@/*` → `src/*`。标签为标记联合（`kind`：`terminal` | `editor` | `preview` | `markdown` | `ai-diff` | `git-diff` | `git-history` | `git-commit-file`），切换时**不卸载**，用 `invisible pointer-events-none` 隐藏，PTY 与开发服务器在后台继续流式输出。

`App.tsx` 串联各模块，保持协调者角色。新功能放进对应 `modules/<area>/`。

### 模块布局（`src/modules/`）

各模块自包含，经 `index.ts` 薄导出，hooks 在 `lib/` 下。

- **terminal/**：`TerminalStack` 经 `useTerminalSession` + `pty-bridge` 为每个标签保持挂载一个 xterm。`osc-handlers.ts` 解析 OSC 7（含 Windows 盘符规范化：`/C:/Users/foo` → `C:/Users/foo`）和 OSC 133 标记。xterm 调色板由中央主题引擎（`modules/theme`）驱动，非本地表。渲染器槽位池化（`rendererPool.ts`，最多 5）：有前台任务（OSC 133 C..D、agent 信号或 `pty_has_foreground_job`）的隐藏叶子以 `display:none` 暂停渲染保留活网格；空闲隐藏叶子释放槽位但保留缓冲区，仅当另一叶子抢槽时才懒序列化。`DormantRing`（1 MiB，溢出不 reset 终端）只为槽位被抢或从未绑定的叶子缓冲字节。绝不在命令中途序列化叶子：在快照上重放增量 TUI 重绘曾 wipe Claude Code。
- **editor/**：CodeMirror 6 栈（`EditorStack` 镜像 `TerminalStack`）。`extensions.ts` 配置语言模式；支持 vim。缓冲区在 LF 空间，保存时恢复原 EOL（`lib/eol.ts`，多数票检测）；缩进单位/tab 经每 pane compartment 按文件检测（`lib/indent.ts`）。保存与 `fs_read_file`/`fs_write_file` 返回的磁盘 mtime 冲突检查（不匹配 → 警告 toast 含显式覆盖，禁止静默最后写入胜出）；外部 format-on-save 仅在自保存快照以来文档未变时应用磁盘读回。超 10 MB 提供「仍要打开」（硬上限 50 MB，`force` 参数）；超 4 MB 关闭语法高亮与 LSP。编辑器标签激活时 Cmd-F 走 CodeMirror 搜索面板（查找/替换/正则），Ctrl-G 打开跳转行；样式在 `chromeTheme.ts`。Format-on-save 在 `lib/externalFormat.ts`（`FORMATTERS` 注册表：biome、prettier、ruff、rustfmt、gofmt、clang-format、shfmt、zig fmt 及自定义 `{file}` 命令模板）；`resolveFormatter` 对每语言覆盖（`editorFormatterByLang`）优先于全局默认。Diff pane 在挂载 CodeMirror 前解析语言：迟到的 compartment 重配置会使 merge view 删除块 widget 无高亮。AI 行内补全（`lib/autocomplete/`）发送缓冲区缩进单位并规范化响应中明确的 tab/空格不匹配（`normalizeIndent.ts`）；触发为 `autocompleteTrigger` 自动或手动，`editor.aiComplete` / `editor.codeComplete` 注册表快捷键（门控在编辑器标签以免终端抢键），Tab 在补全弹窗打开时先接受补全。多行 ghost 首行行内 + 下方块 widget（不用行内 `<br>`）；仅闭合符的行后缀（光标在 `fn(|)` 内）隐藏并在块后重附，使预览等于接受结果；有真实代码的行后缀将 ghost 限制为一行。丢弃与近期前缀重复的建议；多行建议与闭合括号不从以 `;` 结尾的行开始；仅闭合符行从前一行重缩进（`trimSuggestion`/`reindentClosers`，均有测试）。Markdown 编辑为 GFM（`markdownLanguage` 基），围栏代码高亮经共享懒加载语言注册表，Cmd/Ctrl+Click URL，可点任务复选框（`markdownExtras.ts`，均在懒加载 markdown chunk 内；eager-budget 测试强制此约束）。编辑器主题与应用主题解耦：`editorTheme` 为 `"auto" | EditorThemeId`（默认 `"auto"`），`useEditorThemeExt` 经 `resolveEditorThemeId` 解析。`auto` 时编辑器跟随活动应用主题的 `editorTheme[mode]` 配对（实时）；显式选择覆盖。主题 id 与标签在 `settings/store.ts`（`EDITOR_THEMES`/`EDITOR_THEME_LABELS`）；匹配扩展在 `editor/lib/themes.ts`（`EDITOR_THEME_EXT`）。预置 `@uiw` 主题与本地构建主题在 `editor/lib/cmThemes.ts`（Kanagawa wave/lotus/dragon、Everforest、Dracula、Solarized、Catppuccin、Rosé Pine）经 `createTheme`（无额外依赖）。三个 CM 表面（`EditorPane`、`AiDiffPane`、`GitDiffPane`）均经 `useEditorThemeExt` 读主题。
- **explorer/**：文件树，Material/Catppuccin 图标（`iconResolver.ts`），模糊搜索、键盘导航、行内重命名、上下文操作。反斜杠感知 `basename`。
- **preview/**：自动检测开发服务器预览标签（状态栏 pill 在检测到 localhost URL 时建议打开）。
- **tabs/**：`useTabs` 为标签列表 + 活动 id 的单一真相源。`useWorkspaceCwd` 从活动标签派生浏览器根目录与新标签继承 cwd。`basename` 同时按 `/` 和 `\` 分割。
- **header/**：顶栏 + 行内搜索（`SearchInline` 按终端/编辑器适配 `SearchTarget`）。`USE_CUSTOM_WINDOW_CONTROLS` 为 true 时渲染 `WindowControls`（Linux + Windows；macOS 用原生交通灯）。
- **statusbar/**：底栏，`CwdBreadcrumb`（Unix 路径、Windows 盘符、home `~` 段经 `pathUtils.segmentsFromCwd`），AI 工具指示器。
- **shortcuts/**：键位注册表（`shortcuts.ts`）+ `useGlobalShortcuts`。处理器在 `App.tsx`，按 id 传入（`tab.new`、`ai.toggle` 等）。`metaKey || ctrlKey` 跨平台 Cmd/Ctrl。
- **settings/**：设置存储（`store.ts` 经 `tauri-plugin-store`）、偏好 hook、设置窗口打开器。
- **sidebar/**：活动栏 + 可折叠侧边面板（浏览器、源码控制、git 历史）。
- **source-control/**：git 状态/暂存/提交面板与 diff 工作流。
- **git-history/**：提交图轨道、引用、每提交文件 diff。
- **lsp/**：可选语言服务器，未启用时零成本（无进程、无 PATH 检查、eager bundle 外仅 14.5 kB shell）。状态栏 pill 按语言提供启用（找到二进制）或安装（可复制命令）；激活持久化为设置存储中的 `lspActivation`（`enabled`/`dismissed`/未设置）。`sessionManager.ts` 按（服务器、工作区根）键会话，引用计数打开文档，空闲 3 分钟杀，崩溃退避（重 spawn 前冷却；5 分钟内 3 次 → 放弃 + toast 显示服务器 stderr 尾部）。资源不变量：**无根标记则无会话**（dirname 回退曾每目录 spawn 一台服务器并烧掉 GB 级内存），每预设硬上限 4 会话，精简每预设 `initializationOptions`（rust-analyzer：`cachePriming` 关 + 有界 `lru`；tsls：`maxTsServerMemory`）。客户端为懒加载的 `codemirror-languageserver`，子类化（`lib/client.ts`）添加 didClose/didSave/shutdown、`textDocument/references`（Shift-F12；多结果定义与引用共享 `locationsPanel.ts` 选择器）及库遗漏的 publishDiagnostics 能力（无此能力 tsls 不发诊断）；`lib/transport.ts` 桥接到 Rust 管道并应答库忽略的服务器到客户端请求。`vscode-languageserver-protocol` 在 vite.config.ts 中别名到 4 枚举 shim（约省 117 kB）。预设：typescript、rust-analyzer、pyright、ruff、gopls 等；Settings 可配自定义 stdio 服务器。多预设可声明同一语言（pyright 与 ruff 均取 `py`）：`serverForLanguage` 优先已启用候选，故在 pyright 未设置或 dismissed 时启用 ruff 会将 Python 路由到 ruff。WSL 工作区暂排除。
- **markdown/**：Markdown 预览渲染器（支撑 `markdown` 标签类型）。
- **workspace/**：工作区环境切换（本地 + WSL 发行版）。
- **theme/**：自定义主题引擎（无 `next-themes`）。内置预设见 `themes/`，用户主题经 `customThemes.ts` + `validateTheme.ts`，可选背景图。
- **updater/**：基于 `tauri-plugin-updater` 的自动更新 UI。
- **agents/**：内置 Terax Agent 与终端编码 Agent（Claude Code、Codex、Gemini CLI）的通知与管理。共享存储（`store/agentStore.ts`：终端 `sessions` + `localAgent` + `notifications`）与共享路由器（`lib/route.ts`：聚焦且可见时抑制，失焦时 OS 通知，聚焦但隐藏时应用内 Sonner toast）供给顶栏 `NotificationBell`（管理面，Terax Agent 列首，每 Agent hook 启用行）。Toast 用 Sonner（`components/ui/sonner.tsx`），经中央引擎主题化；`lib/agentIcon.tsx` 渲染每 Agent 品牌标记。终端检测在 Rust 侧（`pty/agent_detect.rs`）PTY 读取器字节过滤器上，在 `OSC 133;C;<cmd>` 武装或标记自武装，发出 `terax:agent-signal` 转换（`started`/`working`/`attention`/`finished`/`exited`），仅由 OSC 序列驱动（非原始输出，重绘 TUI 不会抖动），无 Agent 运行时零成本。三 Agent 均收敛到检测器读取的同一 `OSC 777` 标记，经 `agent_enable_hooks(agent)` / `agent_hooks_status(agent)` 在 `modules/agent.rs` 安装（每 Agent 数据驱动 `AgentSpec`；原子写，永不破坏无效 JSON，修剪空组，幂等；门控在 `TERAX_TERMINAL`）。交付因仅 Claude hook 协议可在 hook *响应*中返回终端字节而不同：**Claude**（`~/.claude/settings.json`，`UserPromptSubmit`/`Notification`/`Stop`）经 `terminalSequence` 字段返回标记（旧 3 字段 `notify;Terax;<event>`）。**Codex**（`~/.codex/hooks.json`）与 **Gemini**（`~/.gemini/settings.json`）不能，故 hook *命令*自身发出 4 字段 `notify;Terax;<agent>;<event>` 标记（Unix `printf > /dev/tty`，Windows `terax __terax_notify` 在 `AttachConsole` 后写 `CONOUT$`）并打印 `{}` 作 JSON stdout 空操作。带 Agent 名的标记让自武装在无 preexec 时命名正确 Agent（bash/tmux/Windows）。Terax Agent 路径为 `ai/components/LocalAgentNotificationsBridge.tsx`，将 `chatStore.agentMeta`（`awaiting-approval`→attention，busy→idle→finished，`error`）映射到同一路由器。
- **command-palette/**：模态命令面板（`CommandPalette.tsx`、`commands.ts`）。
- **spaces/**：工作区空间/项目（名称、根、环境、颜色、每空间标签持久化）经 `useSpaces` 与 `SpaceSwitcher`。
- **ai/**：见下文。

### AI 子系统（`src/modules/ai/`）

BYOK。云端经 `@ai-sdk/*`：**OpenAI、Anthropic、Google、xAI、Cerebras、Groq、DeepSeek、Mistral、OpenRouter**，另加 **OpenAI-compatible** 自定义 base URL。本地/离线（密钥可选，运行时提供模型 id）：**LM Studio、MLX、Ollama**。提供商列表在 `config.ts`（`PROVIDERS`）。

- **密钥存储**：OS 钥匙串经 `keyring`（Rust）。前端经 `secrets_*` 读写。服务 `KEYRING_SERVICE = "terax-ai"`。密钥不得落盘、设置存储或 `localStorage`。
- **Agent**（`lib/agent.ts`）：`Experimental_Agent`，`stopWhen: stepCountIs(MAX_AGENT_STEPS)`，系统提示来自 `config.ts`。
- **子 Agent**（`agents/registry.ts`、`agents/runSubagent.ts`）：具独立系统提示与工具子集的命名子 Agent，主 Agent 经 `run_subagent` 调用。
- **会话**（`lib/sessions.ts` + `store/chatStore.ts`）：命名会话，经 `tauri-plugin-store` 持久化于 `terax-ai-sessions.json`。
- **Composer**（`lib/composer.tsx`）：共享输入状态（文本、附件、语音）的 React 上下文。
- **语音输入**：流式转写管道，从 Composer 切换。
- **实时上下文桥**：`App.tsx` 调用 `setLive({ getCwd, getTerminalContext, … })`，工具可读*当前活动*终端的 cwd + 缓冲区最后 300 行。懒设计，不预快照。
- **工具**（`tools/tools.ts`）：`read_file`、`list_directory`、`fs_search`、`fs_grep` 自动执行。`write_file`、`create_directory`、`rename`、`delete`、`run_command`、`shell_session_run`、`shell_bg_spawn` 设 `needsApproval: true`，AI SDK 暂停等待 UI 确认。`lib/security.ts` 为 deny-list，拒绝明显密钥路径（`.env*`、`.ssh/`、凭据、钥匙串目录），读写均应用且不得绕过。
- **编辑 diff**：AI 提议的编辑在并排 diff 标签（`ai-diff`）打开；用户按 hunk 接受/拒绝后写工具才真正执行。
- **Skills / snippets**：可复用提示片段 + 工具包，在 Composer 展示。

### UI 约定

- **shadcn/ui** 已配置（`components.json`，style `radix-luma`，base `mist`，图标库 **hugeicons**）。基元在 `src/components/ui/`，勿手改；升级用 `pnpm dlx shadcn add`。
- **AI Elements**（Vercel）在 `src/components/ai-elements/`，同样规则：重新生成，勿手改。
- **Tailwind v4**：无 `tailwind.config.*`，配置在 `src/App.css` 的 `@theme`。用 `cn()` 来自 `@/lib/utils`。
- 动画：`motion`。可调整布局：`react-resizable-panels`。
- 路径导入：始终 `@/…`，模块间不用相对路径。
- 跨平台路径：来自 OSC 7、浏览器或 OS 的路径，用 `.split(/[\\/]/)` 规范化分隔符。
- 前端规范路径形式为**正斜杠**。`homeDir()` 在 Windows 返回反斜杠；在边界转换（App.tsx setHome）。

### 窗口样式

- macOS：`tauri.conf.json` 中 `titleBarStyle: Overlay` + `hiddenTitle: true`（原生交通灯叠加）。
- Linux：`tauri.linux.conf.json` 中 `decorations: false` + `transparent: true`；realize 后为重断言 GNOME/Mutter CSD。
- Windows：经 `tauri.windows.conf.json` 同 Linux。React 渲染自定义 `WindowControls`。

### Tauri capabilities

`src-tauri/capabilities/default.json` 为 webview 可用插件 API 白名单。新插件通常需要：1. `Cargo.toml` 依赖 2. `lib.rs` `run()` 中 `.plugin(...)` 3. `default.json` 能力项。

### 跨平台约定

- HOME / 缓存目录：用 `dirs` crate（`dirs::home_dir()`、`dirs::cache_dir()`），不用裸 `$HOME` / `%USERPROFILE%`。
- Shell 初始化脚本：Unix 逻辑门控在 `#[cfg(unix)]`；Windows 在 `pty::shell_init::windows`。
- 终端输入：Enter 发 `\r`（CR），非 `\n`（LF），Windows PowerShell 需要 CR。

### 打包配置

- `bundle.targets: "all"` 加 `tauri.conf.json` 各平台节：
  - **macOS**：`minimumSystemVersion: 10.15`。
  - **Linux**：deb 依赖 `libwebkit2gtk-4.1-0`、`libgtk-3-0`；rpm `webkit2gtk4.1`、`gtk3`；AppImage 捆绑媒体框架。
  - **Windows**：NSIS 安装器 `currentUser` 模式（无需管理员），WebView2 经 `embedBootstrapper`（离线安装）。
- 自动更新配置公钥 minisign；发布产物在 `https://github.com/crynta/terax-ai/releases/latest/download/latest.json`。

### 已知陷阱

- **React 19 严格模式**在开发环境双挂载 `useEffect` → 首次渲染终端 spawn 两次。第一个 PTY 几乎立即清理。`SPAWN_LOCK` 互斥串行化；开发日志里 `pty opened id=1` 后跟 `pty closed id=1` 勿慌。
- **Windows PowerShell 进程生命周期**：`portable-pty` 的 `killer.kill()` 只杀直接子进程。后代（如 pwsh 内 `npm run dev`）除非另有机制会存活。`pty/job.rs` 的 Job Object 处理 Terax 进程死亡；显式 `pty_close` 也只杀直接子进程 + 依赖 Job 清理其余。勿在无替代方案时禁用 Job。
- **标签 `cwd` 存储**：来自 OSC 7，正斜杠（`parseOsc7` 剥 `/C:` → `C:` 后）。在 Windows 把 `tab.cwd` 传给 Rust fs 命令的调用方须规范化分隔符或接受两种形式；`pty::shell_init` 的 `apply_common` 处理 PTY spawn；其他调用点自行处理。

## 延伸阅读

长篇贡献者指南在 `docs/` 下，是对 `TERAX.md` 的展开；冲突时以 `TERAX.md` 为准。

- `CONTRIBUTING.md#contributor-guides`：贡献者指南索引
- `docs/architecture/two-process-model.md`：IPC 边界与命令参考
- `docs/architecture/pty-shell-integration.md`：PTY、Shell 初始化、OSC、ConPTY、Job Object
- `docs/architecture/security-model.md`：安全模型与边界
- `docs/architecture/ai-subsystem.md`：AI 栈、会话、工具、添加提供商
- `docs/architecture/terminal-renderer-pool.md`：渲染器池与 DormantRing 不变量
- `docs/contributing/testing.md`：测试约定与核心子系统不变量
