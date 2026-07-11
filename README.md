<div align="center">
  <img src="public/logo.png" width="144" height="144" alt="Terax" />
  <h1>Terax Windows 改造版</h1>

  <p>基于 Terax 上游项目的个人 Windows 改造版本。</p>

  <p>
    <a href="https://github.com/wwwhappyst/terax-ai/releases">下载 Releases</a>
    ·
    <a href="https://github.com/crynta/terax-ai">上游仓库</a>
    ·
    <a href="https://github.com/wwwhappyst/terax-ai">本 Fork 仓库</a>
  </p>
</div>

---

## 项目简介

Terax 是一个基于 Tauri 2、Rust 和 React 19 的 AI 原生终端工作区，包含终端、代码编辑器、文件浏览器、Git 源代码管理、网页预览和 AI 助手等功能。

本仓库是个人 Fork 的 Windows 改造版，主要用于：

- 增加简体中文；
- 改善 Windows 终端快捷键和本地使用体验；
- 适配 windows下的 Agent 通知；
- 保持与作者上游仓库同步，再将个人改动合并到功能分支。

本项目不代表作者上游项目，也不替代上游仓库的正式版本。

## 下载与安装

请前往[本 Fork 的 Releases 页面](https://github.com/wwwhappyst/terax-ai/releases)下载。

当前个人 Release 只提供以下安装包：

- Windows x64：`Terax_*_x64-setup.exe`
- Apple Silicon macOS：`Terax_*_aarch64.dmg`

### Windows

首次运行时，如果 Windows Defender SmartScreen 提示“Windows 已保护你的电脑”，请点击“更多信息”，再点击“仍要运行”。这是因为个人构建没有使用正式代码签名证书。

### macOS

首次打开时，如果 macOS 阻止应用运行，请先确认安装包来源可信。必要时可以在终端执行：

```bash
xattr -dr com.apple.quarantine "/Applications/Terax.app"
```

这只会移除当前应用的隔离标记；重新下载或替换应用后，系统可能再次添加该标记。

## 上游同步

作者上游仓库：<https://github.com/crynta/terax-ai>

同步流程如下：

1. 在 GitHub Fork 页面同步作者仓库的 `main`；
2. 本地快进 `main`；
3. 将更新后的 `main` 合并到 `codex/windows-enhancements`；
4. 解决冲突并完成检查后，再生成个人 Release。

个人版本不会直接安装作者的更新。发现上游有新版本时，请先同步 Fork，再合并 `main`。

## 从源码运行

### 环境要求

- Windows PowerShell 7；
- Node.js 22 或更高版本；
- pnpm 11；
- Rust stable-msvc 工具链；
- Tauri 2 所需的系统依赖。

### 开发运行

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

### 检查

```powershell
pnpm lint
pnpm check-types
pnpm test
```

Rust 检查：

```powershell
Set-Location src-tauri
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

## 构建说明

个人 Release 按上游 GitHub Actions 的平台构建方式生成：

- Windows 使用 Windows 构建机生成 NSIS 安装包；
- Apple Silicon macOS 使用 macOS 构建机生成 `aarch64.dmg`；
- Windows 本机不能原生生成 Apple Silicon macOS 安装包。

详细命令见 [`docs/exec_mode.md`](docs/exec_mode.md)。

## 许可证

Terax 使用 Apache-2.0 许可证。本改造版基于作者的开源项目构建，具体许可信息见 [LICENSE](LICENSE)。
