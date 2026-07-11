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

## Fork Release 打包要求

个人 Fork 的 Release 只发布以下两个安装包，不生成自动更新产物或签名文件：

- Windows x64：`Terax_<version>_x64-setup.exe`
- Apple Silicon：`Terax_<version>_aarch64.dmg`

Windows 本机仅构建 NSIS 安装包：

```powershell
pnpm tauri build --bundles nsis
```

Apple Silicon DMG 由 GitHub Actions 的 `macos-latest` Runner 构建：

```shell
pnpm tauri build --target aarch64-apple-darwin --bundles dmg
```

推送 `v*` 标签会触发 `.github/workflows/release.yml`；该工作流只上传上述两个产物。macOS 安装包未配置 Apple 签名与公证，首次打开时可能出现系统安全提示。

## windows本地调试

```shell
Set-Location D:\dev_source_code\Terax_winpp
$env:CARGO_HOME = 'D:\dev_tools\cargo'
$env:RUSTUP_HOME = 'D:\dev_tools\rustup'
pnpm tauri dev
```
