# Security（中文译本）

> 本文档为 [SECURITY.md](SECURITY.md) 的中文译本，仅供阅读参考。以英文原文为准。

Terax 会运行 Shell、读写文件并与 AI 提供商通信，因此安全漏洞很重要。若发现漏洞，请在公开披露前先告知我们。

## 报告方式

发邮件至 **security@terax.app**。请包含：

- 问题是什么、攻击者可做什么
- 复现步骤（小型 PoC 更佳）
- 版本、操作系统、架构

我们会在几天内回复。修复后会在发布说明中致谢，除非你希望匿名。

请**不要**为安全报告开公开 GitHub Issue。

## 受支持版本

在 `1.0.0` 之前，仅最新 minor 版本获得安全修复。当前版本见 `package.json` 或 [Releases 页面](https://github.com/crynta/terax-ai/releases)。

## 范围内

- `src-tauri/` 中的 Rust 后端（PTY、文件系统、IPC、插件）
- `src/` 中的前端，凡不可信输入进入之处（终端输出、文件内容、AI 工具结果、凭据）
- GitHub 与 `terax.app` 上的发布产物
- 自动更新器

## 范围外

- 上游依赖中的 bug（Tauri、xterm.js、CodeMirror、AI SDK 等），请向上游报告；我们会在上游发布后合入修复。
- 需要已沦陷机器或本地 Shell 访问的攻击者的情况
- 旧版本（`< 0.5`）

## 我们如何保持安全

- **API 密钥**存在 OS 钥匙串（`keyring`），不落盘、不进 `localStorage`、不进日志。
- **无遥测。** Terax 仅在你要求时联网（AI 请求、更新检查、Web 预览）。
- **AI 工具审批。** Agent 的文件写入与 Shell 命令执行前需你确认。
- **渲染器无 Node。** 前端仅通过白名单 Tauri 命令访问宿主。
- **签名发布。** 更新在应用前经验证。

## 我们无法承诺的

- Terax 会运行你（或 Agent）指示它运行的内容，权限与你相同。这就是终端的意义。
- AI 提供商能看到你发送的一切。请阅读其保留策略。
- 本地 LLM 端点（LM Studio、OpenAI-compatible）在网络层被信任，仅将 Terax 指向你控制的服务器。
