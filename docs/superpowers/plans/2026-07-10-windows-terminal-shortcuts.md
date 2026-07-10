# Windows Terminal Shortcuts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 Windows 终端在有选区时使用 `Ctrl+C` 复制、无选区时继续中断，并使用 `Ctrl+V` 粘贴。

**Architecture:** 将快捷键判定放入现有 `keymap.ts` 的纯函数，`rendererPool.ts` 只负责执行复制、粘贴或放行事件。复用现有剪贴板函数和 xterm，不增加依赖或设置项。

**Tech Stack:** TypeScript 6、Vitest 4、xterm 6、Tauri 2。

## Global Constraints

- 仅改变 Windows 行为，macOS 和 Linux 保持不变。
- 保留 `Ctrl+Shift+C` 和 `Ctrl+Shift+V`。
- 不新增依赖，不修改锁文件。
- 新增代码注释必须使用简体中文。
- 每个行为严格执行 RED → GREEN → REFACTOR。

---

### Task 1: 终端剪贴板快捷键判定

**Files:**
- Modify: `src/modules/terminal/lib/keymap.test.ts`
- Modify: `src/modules/terminal/lib/keymap.ts`

**Interfaces:**
- Consumes: `TerminalKeyEvent`。
- Produces: `terminalClipboardAction(event, options): "copy" | "paste" | null`。
- Produces: `TerminalClipboardOptions = { isMac: boolean; isWindows: boolean; hasSelection: boolean }`。

- [ ] **Step 1: 写入 Windows 复制与中断的失败测试**

在 `keymap.test.ts` 的导入中加入 `terminalClipboardAction`，并添加：

```ts
describe("terminalClipboardAction", () => {
  it("copies Ctrl+C only when Windows has a terminal selection", () => {
    const ctrlC = evt({ ctrlKey: true, key: "c", code: "KeyC" });

    expect(
      terminalClipboardAction(ctrlC, {
        isMac: false,
        isWindows: true,
        hasSelection: true,
      }),
    ).toBe("copy");
    expect(
      terminalClipboardAction(ctrlC, {
        isMac: false,
        isWindows: true,
        hasSelection: false,
      }),
    ).toBeNull();
  });
});
```

- [ ] **Step 2: 运行测试并确认 RED**

Run:

```powershell
pnpm test src/modules/terminal/lib/keymap.test.ts
```

Expected: FAIL，因为 `keymap.ts` 尚未导出 `terminalClipboardAction`。

- [ ] **Step 3: 添加最小复制判定**

在 `keymap.ts` 增加：

```ts
export type TerminalClipboardOptions = PlatformOpts & {
  isWindows: boolean;
  hasSelection: boolean;
};

export function terminalClipboardAction(
  event: TerminalKeyEvent,
  options: TerminalClipboardOptions,
): "copy" | "paste" | null {
  if (options.isMac || !event.ctrlKey || event.altKey || event.metaKey) return null;
  const copy = event.code === "KeyC" || event.key.toLowerCase() === "c";
  if (copy && (event.shiftKey || (options.isWindows && options.hasSelection))) {
    return "copy";
  }
  return null;
}
```

同时将 `shiftKey` 加入 `TerminalKeyEvent` 的 `Pick` 字段，并在测试 `evt()` 默认值中加入 `shiftKey: false`。

- [ ] **Step 4: 运行测试并确认 GREEN**

Run: `pnpm test src/modules/terminal/lib/keymap.test.ts`

Expected: PASS。

- [ ] **Step 5: 写入 Windows 粘贴和平台隔离的下一组失败测试**

```ts
it("pastes plain Ctrl+V only on Windows", () => {
  const ctrlV = evt({ ctrlKey: true, key: "v", code: "KeyV" });
  expect(
    terminalClipboardAction(ctrlV, {
      isMac: false,
      isWindows: true,
      hasSelection: false,
    }),
  ).toBe("paste");
  expect(
    terminalClipboardAction(ctrlV, {
      isMac: false,
      isWindows: false,
      hasSelection: false,
    }),
  ).toBeNull();
});

it("keeps Ctrl+Shift+C and Ctrl+Shift+V off macOS", () => {
  const options = { isMac: false, isWindows: false, hasSelection: false };
  expect(
    terminalClipboardAction(
      evt({ ctrlKey: true, shiftKey: true, key: "c", code: "KeyC" }),
      options,
    ),
  ).toBe("copy");
  expect(
    terminalClipboardAction(
      evt({ ctrlKey: true, shiftKey: true, key: "v", code: "KeyV" }),
      options,
    ),
  ).toBe("paste");
});
```

- [ ] **Step 6: 运行测试并确认 RED**

Run: `pnpm test src/modules/terminal/lib/keymap.test.ts`

Expected: FAIL，Windows `Ctrl+V` 尚未返回 `paste`。

- [ ] **Step 7: 添加最小粘贴判定**

在 `terminalClipboardAction` 的 `copy` 判定之后加入：

```ts
const paste = event.code === "KeyV" || event.key.toLowerCase() === "v";
if (paste && (event.shiftKey || options.isWindows)) return "paste";
```

- [ ] **Step 8: 运行测试并确认 GREEN**

Run: `pnpm test src/modules/terminal/lib/keymap.test.ts`

Expected: PASS。

- [ ] **Step 9: 提交纯判定逻辑**

```powershell
git add src/modules/terminal/lib/keymap.ts src/modules/terminal/lib/keymap.test.ts
git commit -m "fix(terminal): 定义 Windows 复制粘贴语义"
```

### Task 2: 将纯判定接入 xterm

**Files:**
- Modify: `src/modules/terminal/lib/rendererPool.ts`

**Interfaces:**
- Consumes: `terminalClipboardAction`。
- Consumes: `IS_MAC`、`IS_WINDOWS`、`readTerminalClipboard`、`writeTerminalClipboard`。
- Produces: xterm 自定义键盘处理行为。

- [ ] **Step 1: 用统一判定替换两个本地快捷键函数**

从 `@/lib/platform` 导入 `IS_MAC`、`IS_WINDOWS`，从 `keymap.ts` 导入 `terminalClipboardAction`。删除 `rendererPool.ts` 内的本地 `IS_MAC`、`isTerminalCopy` 和 `isTerminalPaste`。

在自定义键盘处理器中使用：

```ts
const clipboardAction = terminalClipboardAction(event, {
  isMac: IS_MAC,
  isWindows: IS_WINDOWS,
  hasSelection: slot.term.hasSelection(),
});
if (clipboardAction === "copy") {
  if (event.type === "keydown") {
    const selection = slot.term.getSelection();
    if (selection) void writeTerminalClipboard(selection);
  }
  event.preventDefault();
  return false;
}
if (clipboardAction === "paste") {
  if (event.type === "keydown") {
    const targetLeafId = slot.currentLeafId;
    void readTerminalClipboard().then((text) => {
      if (text && slot.currentLeafId === targetLeafId) slot.term.paste(text);
    });
  }
  event.preventDefault();
  return false;
}
```

- [ ] **Step 2: 运行快捷键和剪贴板测试**

Run:

```powershell
pnpm test src/modules/terminal/lib/keymap.test.ts src/modules/terminal/lib/terminalClipboard.test.ts
pnpm check-types
```

Expected: 两个测试文件 PASS，类型检查退出码为 0。

- [ ] **Step 3: Windows 人工验收**

在 `pnpm tauri dev` 中验证：

1. PowerShell 运行等待命令，无选区按 `Ctrl+C`，命令被中断。
2. 选择终端文本后按 `Ctrl+C`，文本进入剪贴板且 CLI 未被中断。
3. `Ctrl+V` 粘贴中文和多行文本。
4. `Ctrl+Shift+C/V` 仍然可用。

- [ ] **Step 4: 提交 xterm 接入**

```powershell
git add src/modules/terminal/lib/rendererPool.ts
git commit -m "fix(terminal): 接入 Windows 原生复制粘贴快捷键"
```
