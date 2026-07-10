# Windows 环境

## 项目要求

- Node.js：`>=22`。
- pnpm：`11.9.0`。
- Rust：稳定版 MSVC 工具链。
- Visual Studio Build Tools：使用 C++ 的桌面开发工作负载，包含 MSVC x64/x86 工具集和 Windows SDK。
- Microsoft Edge WebView2 Runtime。

官方参考：

- [Terax README](../README.md)
- [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows)

## 安装方式

Rust 使用官方 Windows 安装器，默认工具链设为 `stable-msvc`。pnpm 使用 Node 自带 Corepack：

```powershell
corepack enable
corepack prepare pnpm@11.9.0 --activate
```

Visual Studio Installer 中选择“使用 C++ 的桌面开发”。WebView2 缺失时安装 Microsoft Evergreen Bootstrapper。

本项目同时生成 NSIS 与 MSI；只有 MSI 打包提示 `failed to run light.exe` 时，才在 Windows 功能中启用 VBSCRIPT。

## 本机核验（2026-07-10）

| 项目 | 状态 |
| --- | --- |
| Git | 2.54.0 |
| Node.js | 24.16.0 |
| pnpm | 11.9.0 |
| Rust / Cargo / rustup | stable-msvc；工具链与缓存位于 `D:\\dev_tools\\rustup`、`D:\\dev_tools\\cargo` |
| MSVC C++ 工具集与 Windows SDK | 已安装，`cl.exe` 已验证 |
| WebView2 Runtime | 150.0.4078.48 |

WSL 是可选工作区环境，不是本项目在 Windows 本地开发的前置条件。
