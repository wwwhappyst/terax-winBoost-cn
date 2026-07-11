# Contributing to Terax（中文译本）

> 本文档为 [CONTRIBUTING.md](CONTRIBUTING.md) 的中文译本，仅供阅读参考。以英文原文为准。

Terax 由单人维护，产品方向明确。欢迎贡献，但**方向对齐比数量更重要**。

本文帮助你判断*是否*以及*如何*贡献，提高合并概率，避免双方浪费时间。

## 项目如何运作

- Terax 有一名活跃维护者 ([@crynta](https://github.com/crynta))。
- 审阅带宽有限。
- 并非每项技术上正确的贡献都会被接受。与项目方向的对齐与代码质量同等重要。
- 范围与方向见 [ROADMAP.md](ROADMAP.md)（中文：[ROADMAP.zh-CN.md](ROADMAP.zh-CN.md)）。开任何非琐碎事项前先读。

这对单人项目很正常。PR 被拒不是针对个人。

## 快速开始

```bash
pnpm install
pnpm tauri dev
```

前置：Rust（stable）、Node 20+、pnpm，以及各平台 [Tauri 前置依赖](https://tauri.app/start/prerequisites/)。

架构与安全贡献方式见 [TERAX.md](TERAX.md)（中文：[TERAX.zh-CN.md](TERAX.zh-CN.md)）及下方 [贡献者指南](#contributor-guides)。

## Contributor guides

`docs/` 下的长篇指南是对 [TERAX.md](TERAX.md) 的展开。与 `TERAX.md` 冲突时，以 `TERAX.md` 为准。

### 架构指南

- [双进程模型与 IPC 命令参考](docs/architecture/two-process-model.md) - Rust 拥有全部 OS 访问；webview 经 `invoke()` 通信。命令目录与如何新增命令。
- [PTY Shell 集成](docs/architecture/pty-shell-integration.md) - PTY 会话、Shell 初始化脚本、OSC 7 / 133、ConPTY、SPAWN_LOCK、Job Object、WSL。
- [安全模型](docs/architecture/security-model.md) - deny-list、SSRF 防护、工作区授权、AI 工具审批、IPC 白名单、OSC 信任、钥匙串处理。
- [AI 子系统](docs/architecture/ai-subsystem.md) - 提供商、Agent、子 Agent、会话、Composer、工具、编辑 diff、实时上下文桥。含添加新提供商的 walkthrough。
- [终端渲染器池](docs/architecture/terminal-renderer-pool.md) - 槽位池化、DormantRing、禁止命令中途序列化的不变量。

### 贡献指南

- [测试](docs/contributing/testing.md) - 测试约定、如何运行检查、何谓好的核心子系统测试。

## 讨论渠道

Discord：[Crynta OS](https://discord.gg/tyveTUyEp7)

设计讨论、范围问题、「我该做 X 吗？」、快速反馈用 Discord。具体 bug 与功能跟踪用 GitHub Issues。

## 什么样的贡献更好

这些更容易快速合并：

- **Bug 修复**，复现步骤清晰。
- **文档/错别字/小 UX 修复** - 直接开 PR。
- **事先讨论过的功能** - 先在 Issue 或 Discord 对齐。
- **小而聚焦的改动** - 易审、低风险。

若改动小而明显（错别字、窄 bugfix、小文档），直接开 PR，无需 Issue。

## 保持改动聚焦

**只改达成 stated 目标所需的内容。**

若在修 `terminal.tsx` 的 bug，不要同时：

- 重排其他文件格式
- 清理无关代码
- 修未触及文件的 lint
- 在一个 PR 里合多个无关修复

即便这些是「改进」，也会让审阅变难、拖慢一切。若要清理，讨论后另开 PR。

**一个 PR = 一个逻辑变更。** 多主题 PR 会被要求拆分。

## 先讨论（较大改动必需）

除小修复外，**开 PR 前必须先讨论**。包括：

- 新功能
- UI/UX 或默认行为变更
- 重构或「清理」
- 性能重写
- 架构变更
- 触及多文件/多系统
- 新 AI 提供商

含大量未经请求的显著改动的 PR 会在无详细审阅的情况下关闭。这不是为了打击贡献，而是确保大量工作投入前已对齐。

十分钟对话可省下一条不符合路线图的 500 行 PR。

## 质量门槛

Terax 定位为**轻量、快速、生产级**。每个 PR 按以下标准审阅：

- `pnpm lint` 通过
- `pnpm check-types` 通过
- `pnpm test` 通过
- `cargo clippy --all-targets --locked -- -D warnings` 通过
- `cargo nextest run --locked` 通过（或 `cargo test --locked`）
- 推送前已 `cargo fmt`
- 已知热路径无性能回退：终端渲染器、PTY 流、AI 流式、源码控制、文件浏览器
- 无未经论证的新重依赖（客户端 bundle gzip >50KB，Rust 编译 >5MB）
- 保持平台对等（macOS / Linux / Windows / WSL 仍可用）
- 触及 AI 工具面、文件系统访问、网络路径、IPC 命令的变更需安全审阅

不确定如何测性能或何谓热路径，在 Discord 或 Issue 问。确认比被拒好。

## 核心子系统改动需要测试

PR 弄坏 Terax 最常见方式是**局部修复、全局爆炸半径**：diff 解决一个报告案例，读起来没问题，类型检查与 clippy 都过，却在同子系统其他所有场景静默破坏。仅靠审阅抓不到。测试可以。

因此若改动触及以下承重路径的行为，PR 必须新增或扩展测试以锁定你依赖的不变量：

- **Shell/终端启动**：启动哪个 Shell、cwd、环境、登录标志。此处「修复」可能让终端完全起不来。
- **工作区授权**：spawn、git、AI 工具可操作哪些目录。允许与拒绝两侧都要测。
- **Git 命令层**：仓库根解析、pathspec/参数防护、状态解析。
- **文件系统变更**：原子写、符号链接处理、部分失败无数据丢失。
- **IPC 命令面与 AI 工具面**：webview 或 Agent 可 invoke 的一切。
- **影响面大的纯逻辑**：cwd 继承、标签/分屏树变换、OSC/提示符解析、命令防护。

测试门槛是真实覆盖契约，非占位。测会真坏的情况：边界、拒绝路径、「home 上一级会怎样」。若不知如何测，开 PR 前在 Discord 问。通常比 revert 短。

UI 渲染、主题、语法高亮表、类型检查器已保证的不需要测试。

## Terax 不是什么

设定期望：

- Terax 不试图成为完整 IDE 替代品（VS Code、Cursor、Zed）。
- 不做：完整 LSP、Jupyter 笔记本、集成调试器 UI、包管理器 UI、完整 Web 浏览器。
- 这不是 curated「首次开源贡献」项目。新手欢迎，但审阅正常标准。
- 机械重构、大范围风格变更、顺手重写无益。
- 欢迎 AI 辅助贡献，但 PR 须体现对现有模式的理解。作者未读的低质量 AI 生成代码会被关闭。

## 分支

从 `main` 拉分支。前缀（kebab-case）：

| 前缀       | 用途                                     |
| ---------- | ---------------------------------------- |
| `feat/`    | 新功能                                   |
| `fix/`     | Bug 修复                                 |
| `chore/`   | 重构、工具、配置、依赖                   |
| `docs/`    | 仅文档                                   |
| `perf/`    | 性能                                     |
| `security/`| 安全修复或加固                             |

示例：`feat/split-panes`、`fix/explorer-focus`、`security/path-guard`。

不要从 fork 的 `main` 开 PR。用功能分支。

## 提交与 PR

对多数 PR，**PR 标题会成为 squash 提交**。多提交 PR 若原子提交良好，维护者可酌情 merge commit（安全审计、多步重构）。标题须遵循 [Conventional Commits](https://www.conventionalcommits.org/)：

```
feat(terminal): add split panes
fix(explorer): prevent input from disappearing on create
chore(deps): bump tauri to 2.x
security(ai): tighten path guard
```

类型：`feat`、`fix`、`chore`、`docs`、`perf`、`refactor`、`test`、`build`、`ci`、`security`。

常见 scope：`terminal`、`editor`、`explorer`、`pty`、`ai`、`agents`、`settings`、`tabs`、`shortcuts`、`ui`、`git`、`preview`、`windows`、`linux`、`macos`、`wsl`。

PR 内单个提交信息可自由（会被 squash 或分组）。

**填写 PR 模板。** 包含：改了什么、为什么、如何测试。UI 变更附截图/GIF。「手动测试了…」是最低要求。

若想中途反馈，**尽早开 draft PR**。完成后再标「Ready for review」。

### 更容易合并的

- 问题陈述清晰
- diff 小而聚焦
- 遵循现有模式（写代码前读 2-3 个邻近文件）
- 类型检查 / lint / 测试全过
- 手动测试说明写清步骤

### 容易被退回的

- 多主题 PR
- 无事先讨论的大架构 PR
- 无论证的新依赖
- 无迁移说明的破坏性变更
- 与改动无关的顺手格式化
- 明显作者未读的 AI 生成代码

## 代码风格

- 遵循现有模式。新增前读 2-3 个相邻文件。
- TypeScript：除非真需要，不用 `any`。严格模式开启。
- Rust：`cargo fmt` + `clippy` 干净。
- 注释：只写*为何*，不写*做什么*。代码自解释。不要多段 docstring。
- 代码与提交信息不用 emoji。
- 面向用户的字符串用美式英语。

## 项目布局

```
src-tauri/                  Rust 后端
  src/
    lib.rs                  Tauri 命令注册
    modules/
      agent.rs              终端编码 Agent hook 安装/状态
      fs/                   文件系统命令（读/写/搜索/grep）
      git/                  源码控制命令
      history/              Shell 历史集成
      mod.rs                模块导出
      net.rs                带 SSRF 防护的 AI HTTP 代理
      proc.rs               进程工具
      pty/                  终端会话、Shell 集成、DA 过滤
      secrets.rs            OS 钥匙串访问
      shell/                一次性/会话/后台 Shell 命令
      workspace.rs          WSL 桥、工作区环境、授权注册表

src/                        React 前端
  App.tsx                   顶层协调者
  components/               shadcn/ui + AI Elements
  modules/
    agents/                 Agent 通知与管理
    ai/                     Agent、会话、工具、提供商、Composer
    command-palette/        模态命令面板与操作
    editor/                 CodeMirror 栈、AI 自动补全
    explorer/               文件树
    git-history/            Git 图与历史面板
    header/                 顶栏、搜索、窗口控件
    markdown/               Markdown 预览渲染器
    preview/                开发服务器、图片、Web 预览
    settings/               设置 UI 与偏好存储
    shortcuts/              键位注册表
    sidebar/                活动栏与侧边面板
    source-control/         源码控制面板
    spaces/                 工作区空间/项目与每空间标签持久化
    statusbar/              底栏与 cwd 面包屑
    tabs/                   标签/分屏模型
    terminal/               xterm.js 会话、OSC 处理器、渲染器池
    theme/                  自定义主题引擎与预设
    updater/                自动更新 UI
    workspace/              工作区环境切换
```

## FAQ

**问：修错别字或明显 bug 要先问吗？**
答：不用，直接开 PR。

**问：我有新功能想法。**
答：开 GitHub Issue 或到 Discord。未经讨论不要开 PR。

**问：PR 无详细反馈就被关了。**
答：通常表示与项目方向不对齐，或范围太大无法负责任地审阅。单人项目很正常。若愿意缩小范围重做，欢迎 reopen。

**问：我能做 open issue 吗？**
答：先评论确认仍相关且无人在做。非琐碎事项实现前先讨论方案。

**问：修 bug 时看到能写得更干净的代码。**
答：聚焦 stated 目标。若清理重要，讨论后另开 PR。

**问：审阅要多久？**
答：视情况而定。小 bug 或文档：通常几天内。大功能：可能一两周。事先讨论过的更快。

**问：新 AI 提供商 PR 为什么被关？**
答：多数提供商请求现可由 `openai-compatible`（指向任意 OpenAI 兼容 base URL）或 OpenRouter 覆盖。新内置提供商须论证超出既有覆盖的独特价值。

**问：main 前进后 PR 冲突了，要 rebase 吗？**
答：若改动仍相关且 reasonably 小，要。若是大而陈旧的 PR，可能被关并建议 rebase 后 reopen。不是针对个人，是节奏问题。

## 安全问题

不要公开 Issue。见 [SECURITY.md](SECURITY.md)（中文：[SECURITY.zh-CN.md](SECURITY.zh-CN.md)）。

## 许可证

贡献即表示你同意作品以 [Apache-2.0](LICENSE) 许可。无需 CLA。
