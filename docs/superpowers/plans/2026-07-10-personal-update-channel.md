# Personal Update Channel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 防止个人定制版从原作者更新源下载并安装覆盖当前功能。

**Architecture:** 在个人签名 Release 建立前，不挂载现有 `UpdaterDialog`，因此不会执行启动更新检查。保留上游更新代码，便于以后改成个人仓库签名更新源；不删除依赖或修改锁文件。

**Tech Stack:** React 19、Tauri updater plugin 2、GitHub Releases。

## Global Constraints

- 不修改依赖和锁文件。
- 不伪造自动更新签名或复用原作者私钥。
- 未建立个人签名 Release 前不得调用原作者更新端点。
- 这是单行集成配置，不为源码是否包含某字符串创建脆弱单元测试。

---

### Task 1: 停止挂载官方更新入口

**Files:**
- Modify: `src/app/App.tsx`

**Interfaces:**
- Removes consumption of: `UpdaterDialog`。
- Keeps: `src/modules/updater` 和现有依赖，等待个人更新源启用。

- [ ] **Step 1: 建立修改前证据**

Run:

```powershell
rg -n 'UpdaterDialog' src/app/App.tsx
```

Expected: 输出导入位置和 JSX 挂载位置。

- [ ] **Step 2: 删除导入和 JSX 挂载**

从 `src/app/App.tsx` 删除：

```ts
import { UpdaterDialog } from "@/modules/updater";
```

并删除：

```tsx
<UpdaterDialog />
```

- [ ] **Step 3: 验证更新入口不再进入主窗口**

Run:

```powershell
if (rg -n 'UpdaterDialog' src/app/App.tsx) { throw '主窗口仍挂载官方更新入口' }
pnpm check-types
pnpm test src/app/eager-budget.test.ts
```

Expected: `rg` 无匹配，类型检查和 eager budget 测试退出码为 0。

- [ ] **Step 4: 提交个人构建保护**

```powershell
git add src/app/App.tsx
git commit -m "chore(updater): 禁用个人构建的官方更新入口"
```

### Task 2: 将来启用个人签名更新源

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `.github/workflows/release.yml`
- Modify: `src/app/App.tsx`

**Interfaces:**
- Consumes: 个人仓库签名私钥对应的公钥和 `wwwhappyst/terax-ai` Release。
- Produces: 只接受个人签名的更新清单。

- [ ] **Step 1: 在拥有个人 Tauri 签名密钥后修改更新配置**

将 updater endpoint 改为：

```json
"endpoints": [
  "https://github.com/wwwhappyst/terax-ai/releases/latest/download/latest.json"
]
```

并把 `pubkey` 改为个人签名私钥对应的公钥。私钥和密码只放 GitHub Actions Secrets，不写仓库、不输出日志。

- [ ] **Step 2: 恢复主窗口更新入口**

重新导入并挂载：

```tsx
import { UpdaterDialog } from "@/modules/updater";
```

```tsx
<UpdaterDialog />
```

- [ ] **Step 3: 验证更新配置只指向个人仓库**

Run:

```powershell
$config = Get-Content src-tauri/tauri.conf.json -Raw -Encoding UTF8 | ConvertFrom-Json
$endpoint = $config.plugins.updater.endpoints[0]
if ($endpoint -ne 'https://github.com/wwwhappyst/terax-ai/releases/latest/download/latest.json') { throw "更新源错误: $endpoint" }
if (-not $config.plugins.updater.pubkey) { throw '缺少个人更新公钥' }
```

Expected: 命令退出码为 0，且不会输出任何私钥或密码。

- [ ] **Step 4: 运行完整构建验证**

Run:

```powershell
pnpm check-types
pnpm test
pnpm tauri build
```

Expected: 全部退出码为 0，生成的 `latest.json` 指向个人 Release 资产。

- [ ] **Step 5: 提交个人更新通道**

```powershell
git add src-tauri/tauri.conf.json .github/workflows/release.yml src/app/App.tsx
git commit -m "feat(updater): 使用个人签名更新通道"
```
