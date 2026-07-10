# Terax Windows 改造规则

## 项目

Terax 是基于 Tauri 2、Rust、React 19 和 TypeScript 的桌面终端应用。本仓库以 Windows 支持为当前改造重点。

## 开发约束

- 所有说明和新增代码注释使用简体中文。
- Windows 命令使用 PowerShell 7，并以 UTF-8 读写文本。
- 使用 pnpm，不使用 npm、npx 或 yarn。
- Node.js 保持 `>=22`，Rust 使用 stable-msvc 工具链。
- Rust 工具链与缓存目录为 `D:\\dev_tools\\rustup` 和 `D:\\dev_tools\\cargo`。
- 保持 Windows、macOS 和 Linux 的现有行为；平台差异放在正确的 Rust `cfg` 分支中。
- 修改前先读取相关代码和全部调用方；只做当前需求所需的最小改动，不引入无关依赖或重构。
- 未经明确确认，不安装新依赖、不构建发布包、不推送或部署。

## 文档入口

- `TERAX.md`：架构与项目约定的来源。
- `docs/envs.md`：Windows 环境要求、安装方式和本机核验。
- `docs/exec_mode.md`：依赖安装、开发、检查和打包命令。
- `docs/architecture/`：子系统架构说明。

`AGENTS.md` 只记录长期规则、项目简介和文档入口，不记录单次进度或验证结果。
