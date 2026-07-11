# 执行命令

在项目根目录 `D:\dev_source_code\Terax_winpp` 的 PowerShell 7 中执行。

## 安装项目依赖

```powershell
pnpm install --frozen-lockfile
```

## 开发运行

```shell
pnpm tauri dev
```

## 检查

```shell
pnpm lint
pnpm check-types
pnpm test
Set-Location src-tauri
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

需要与 CI 一致时，先安装 `cargo-nextest`，再执行：

```powershell
Set-Location src-tauri
cargo nextest run --locked
```

## 生产打包

```shell
pnpm tauri build
```

Windows 安装包输出到 `src-tauri/target/release/bundle/`。

## windows本地调试

```shell
Set-Location D:\dev_source_code\Terax_winpp
$env:CARGO_HOME = 'D:\dev_tools\cargo'
$env:RUSTUP_HOME = 'D:\dev_tools\rustup'
pnpm tauri dev
```