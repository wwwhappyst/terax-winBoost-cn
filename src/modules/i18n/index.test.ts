// 验证轻量翻译层在中文查询、英文回退和动态参数上的预期行为。
import { describe, expect, it } from "vitest";

type I18nModule = {
  translate?: (
    language: "en" | "zh-CN",
    text: string,
    params?: Record<string, string | number>,
  ) => string;
};

async function loadI18n(): Promise<I18nModule | null> {
  // 动态加载让尚未实现的模块表现为可读断言失败，而不是收集阶段报错。
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

  it("translates the interface language setting", async () => {
    const module = await loadI18n();
    expect(module?.translate, "i18n module must export translate").toBeTypeOf(
      "function",
    );
    if (!module?.translate) return;

    expect(module.translate("zh-CN", "Interface language")).toBe("界面语言");
    expect(
      module.translate(
        "zh-CN",
        "Choose the language used by the Terax interface.",
      ),
    ).toBe("选择 Terax 界面使用的语言。");
  });
});
