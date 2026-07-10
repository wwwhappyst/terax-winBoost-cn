# 界面国际化实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 增加 English/简体中文实时切换，并让正常可见界面通过一个无依赖翻译层取得文本。

**Architecture:** 英文原文作为翻译键和默认回退，`zh-CN.ts` 集中保存中文。语言偏好复用现有 Zustand/LazyStore 跨窗口同步；组件使用 Hook，命令式 Toast 使用读取当前 Store 的函数。

**Tech Stack:** React 19、TypeScript 6、Zustand 5、Vitest 4、浏览器原生 `Intl`。

## Global Constraints

- 只支持 `en` 和 `zh-CN`，默认 `en`。
- 不翻译终端、Agent/AI 输出、技术日志、Rust 错误、模型名和路径。
- 不使用 DOM 扫描、MutationObserver 或第三方 i18n 依赖。
- 缺少翻译必须回退英文，不得抛错。
- 新增文件和函数必须包含必要的简体中文注释。

---

### Task 1: 纯翻译函数

**Files:**
- Create: `src/modules/i18n/index.test.ts`
- Create: `src/modules/i18n/index.ts`
- Create: `src/modules/i18n/zh-CN.ts`

**Interfaces:**
- Produces: `AppLanguage = "en" | "zh-CN"`。
- Produces: `TranslationParams = Record<string, string | number>`。
- Produces: `translate(language, text, params?): string`。

- [ ] **Step 1: 写入翻译、回退和占位符的失败测试**

创建 `src/modules/i18n/index.test.ts`：

```ts
// 验证轻量翻译层的英文回退、中文查询和动态参数替换。
import { describe, expect, it } from "vitest";

type I18nModule = {
  translate?: (
    language: "en" | "zh-CN",
    text: string,
    params?: Record<string, string | number>,
  ) => string;
};

async function loadI18n(): Promise<I18nModule | null> {
  const modulePath = "./index.ts";
  try {
    return (await import(/* @vite-ignore */ modulePath)) as I18nModule;
  } catch {
    return null;
  }
}

describe("translate", () => {
  it("returns Chinese for a known key and English for a missing key", async () => {
    const module = await loadI18n();
    expect(module, "i18n module must exist").not.toBeNull();
    expect(module?.translate, "i18n module must export translate").toBeTypeOf(
      "function",
    );
    if (!module?.translate) return;
    expect(module.translate("zh-CN", "Open settings")).toBe("打开设置");
    expect(module.translate("zh-CN", "Untranslated upstream text")).toBe(
      "Untranslated upstream text",
    );
    expect(module.translate("en", "Open settings")).toBe("Open settings");
  });

  it("replaces string and numeric placeholders", async () => {
    const module = await loadI18n();
    expect(module?.translate, "i18n module must export translate").toBeTypeOf(
      "function",
    );
    if (!module?.translate) return;
    expect(
      module.translate("zh-CN", "Copied {count} files", { count: 2 }),
    ).toBe("已复制 2 个文件");
  });
});
```

- [ ] **Step 2: 运行测试并确认 RED**

Run: `pnpm test src/modules/i18n/index.test.ts`

Expected: FAIL，断言提示 `i18n module must exist`，不是无法解析静态导入。

- [ ] **Step 3: 添加最小中文词典**

创建 `src/modules/i18n/zh-CN.ts`：

```ts
// 集中保存简体中文界面翻译；英文原文同时作为稳定翻译键。
export const ZH_CN: Record<string, string> = {
  "Open settings": "打开设置",
  "Copied {count} files": "已复制 {count} 个文件",
};
```

- [ ] **Step 4: 添加最小翻译函数**

创建 `src/modules/i18n/index.ts`：

```ts
// 提供无依赖的界面翻译查询，并在缺少中文时安全回退英文。
import { ZH_CN } from "./zh-CN";

export type AppLanguage = "en" | "zh-CN";
export type TranslationParams = Record<string, string | number>;

export function translate(
  language: AppLanguage,
  text: string,
  params: TranslationParams = {},
): string {
  const template = language === "zh-CN" ? (ZH_CN[text] ?? text) : text;
  return template.replace(/\{(\w+)\}/g, (match, key: string) =>
    Object.hasOwn(params, key) ? String(params[key]) : match,
  );
}
```

- [ ] **Step 5: 运行测试并确认 GREEN**

Run: `pnpm test src/modules/i18n/index.test.ts`

Expected: 2 tests PASS。

- [ ] **Step 6: 提交翻译核心**

```powershell
git add src/modules/i18n
git commit -m "feat(i18n): 添加轻量界面翻译核心"
```

### Task 2: 持久化语言偏好

**Files:**
- Create: `src/modules/settings/appLanguage.test.ts`
- Modify: `src/modules/settings/store.ts`
- Modify: `src/modules/settings/preferences.ts`

**Interfaces:**
- Consumes: `AppLanguage`。
- Produces: `Preferences.language: AppLanguage`。
- Produces: `coerceAppLanguage(value: unknown): AppLanguage`。
- Produces: `setLanguage(value: AppLanguage): Promise<void>`。

- [ ] **Step 1: 写入无效语言回退的失败测试**

```ts
// 验证持久化数据只能产生支持的界面语言。
import { describe, expect, it } from "vitest";
import { coerceAppLanguage } from "./store";

describe("coerceAppLanguage", () => {
  it("keeps supported languages and falls back to English", () => {
    expect(coerceAppLanguage("en")).toBe("en");
    expect(coerceAppLanguage("zh-CN")).toBe("zh-CN");
    expect(coerceAppLanguage("zh-TW")).toBe("en");
    expect(coerceAppLanguage(null)).toBe("en");
  });
});
```

- [ ] **Step 2: 运行测试并确认 RED**

Run: `pnpm test src/modules/settings/appLanguage.test.ts`

Expected: FAIL，因为 `coerceAppLanguage` 尚未导出。

- [ ] **Step 3: 在现有 Store 中增加语言字段**

在 `store.ts` 中使用 `import type { AppLanguage } from "@/modules/i18n"`，并加入：

```ts
const KEY_LANGUAGE = "language";

export function coerceAppLanguage(value: unknown): AppLanguage {
  return value === "zh-CN" ? "zh-CN" : "en";
}

export async function setLanguage(value: AppLanguage): Promise<void> {
  await writePref(KEY_LANGUAGE, value);
}
```

在现有 `Preferences` 类型和 `DEFAULT_PREFERENCES` 对象中分别插入以下字段，不改动其他字段：

```ts
export type Preferences = {
  language: AppLanguage;
};

export const DEFAULT_PREFERENCES: Preferences = {
  language: "en",
};
```

`loadPreferences()` 使用 `coerceAppLanguage(get(KEY_LANGUAGE))`，`onPreferencesChange()` 的映射加入 `[KEY_LANGUAGE]: "language"`。

- [ ] **Step 4: 运行测试并确认 GREEN**

Run:

```powershell
pnpm test src/modules/settings/appLanguage.test.ts
pnpm check-types
```

Expected: 测试 PASS，类型检查退出码为 0。

- [ ] **Step 5: 提交语言偏好**

```powershell
git add src/modules/settings/store.ts src/modules/settings/preferences.ts src/modules/settings/appLanguage.test.ts
git commit -m "feat(settings): 持久化界面语言"
```

### Task 3: React 和命令式翻译入口

**Files:**
- Modify: `src/modules/i18n/index.ts`
- Modify: `src/main.tsx`
- Modify: `src/settings/main.tsx`

**Interfaces:**
- Produces: `useTranslation(): (text: string, params?: TranslationParams) => string`。
- Produces: `t(text, params?): string`，用于非 React 调用。

- [ ] **Step 1: 添加读取当前偏好的翻译入口**

在 `index.ts` 中加入：

```ts
import { useCallback } from "react";
import { usePreferencesStore } from "@/modules/settings/preferences";

export function useTranslation() {
  const language = usePreferencesStore((state) => state.language);
  return useCallback(
    (text: string, params?: TranslationParams) =>
      translate(language, text, params),
    [language],
  );
}

export function t(text: string, params?: TranslationParams): string {
  return translate(usePreferencesStore.getState().language, text, params);
}
```

- [ ] **Step 2: 同步根元素语言**

在主窗口和设置窗口初始化偏好后，根据 `language` 设置：

```ts
document.documentElement.lang = language;
```

语言变化时重复设置，两个窗口均不得依赖重启。

- [ ] **Step 3: 运行核心测试和类型检查**

Run:

```powershell
pnpm test src/modules/i18n/index.test.ts src/modules/settings/appLanguage.test.ts
pnpm check-types
```

Expected: 全部 PASS。

- [ ] **Step 4: 提交 React 接入**

```powershell
git add src/modules/i18n/index.ts src/main.tsx src/settings/main.tsx
git commit -m "feat(i18n): 接入实时语言状态"
```

### Task 4: 设置页语言选择器

**Files:**
- Modify: `src/settings/sections/GeneralSection.tsx`
- Modify: `src/modules/i18n/zh-CN.ts`

**Interfaces:**
- Consumes: `language`、`setLanguage`、`useTranslation`。

- [ ] **Step 1: 在 General 页顶部增加语言选择**

使用项目现有 `Select`，值固定为 `en` 和 `zh-CN`：

```tsx
<SettingRow
  title={tr("Interface language")}
  description={tr("Choose the language used by the Terax interface.")}
>
  <Select value={language} onValueChange={(value) => void setLanguage(value as AppLanguage)}>
    <SelectTrigger size="sm" className="h-8 w-36 text-[12px]">
      <SelectValue />
    </SelectTrigger>
    <SelectContent>
      <SelectItem value="en">English</SelectItem>
      <SelectItem value="zh-CN">简体中文</SelectItem>
    </SelectContent>
  </Select>
</SettingRow>
```

词典加入：

```ts
"Interface language": "界面语言",
"Choose the language used by the Terax interface.": "选择 Terax 界面使用的语言。",
```

- [ ] **Step 2: 运行类型检查并人工验证双窗口同步**

Run: `pnpm check-types`

Expected: 退出码为 0。人工切换语言后，主窗口和设置窗口的 `html[lang]` 同步变化。

- [ ] **Step 3: 提交语言选择器**

```powershell
git add src/settings/sections/GeneralSection.tsx src/modules/i18n/zh-CN.ts
git commit -m "feat(settings): 添加界面语言选择"
```

### Task 5: 迁移正常可见界面

**Files:**
- Modify: `src/app/**/*.tsx`
- Modify: `src/settings/**/*.tsx`
- Modify: `src/modules/**/*.tsx`
- Modify: `src/components/WindowControls.tsx`
- Modify: `src/modules/i18n/zh-CN.ts`

**Interfaces:**
- Consumes: React 组件中的 `useTranslation` 和命令式路径中的 `t`。

- [ ] **Step 1: 按可独立验收的界面组迁移**

依次处理以下组，每组单独提交，禁止混入逻辑重构：

1. `src/settings/SettingsApp.tsx` 与 `src/settings/sections/*.tsx`。
2. `src/app/App.tsx`、`src/app/components/*.tsx`、`src/modules/tabs`、`src/modules/header`。
3. `src/modules/explorer`、`src/modules/editor`、`src/modules/preview`。
4. `src/modules/agents`、`src/modules/ai/components`、`src/modules/updater`。
5. 其余包含用户可见英文的 `src/**/*.tsx`。

组件统一采用：

```tsx
const tr = useTranslation();
return <Button title={tr("Open settings")}>{tr("Open")}</Button>;
```

命令式 Toast 统一采用：

```ts
toast.success(t("Settings saved"));
```

- [ ] **Step 2: 每组迁移后运行验证**

Run:

```powershell
pnpm test src/modules/i18n/index.test.ts src/modules/settings/appLanguage.test.ts
pnpm check-types
pnpm lint
```

Expected: 全部退出码为 0。

- [ ] **Step 3: 扫描遗漏的可见英文**

Run:

```powershell
rg -n --glob '*.tsx' '>[[:space:]]*[A-Za-z][^<{]*<|title="[A-Za-z]|placeholder="[A-Za-z]|aria-label="[A-Za-z]' src
```

逐项确认结果属于已翻译文本、协议/品牌/模型/路径等明确排除项；其余必须通过 `tr()` 迁移并加入 `zh-CN.ts`。

- [ ] **Step 4: 完整前端验证**

Run:

```powershell
pnpm test
pnpm check-types
pnpm lint
```

Expected: 全部退出码为 0，中文模式无空文本或翻译键直出。
