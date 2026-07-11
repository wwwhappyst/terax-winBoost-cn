// 验证持久化数据只能产生当前支持的界面语言。
import { describe, expect, it } from "vitest";
import * as store from "./store";

describe("coerceAppLanguage", () => {
  it("keeps supported languages and falls back to English", () => {
    const coerce = (
      store as unknown as {
        coerceAppLanguage?: (value: unknown) => "en" | "zh-CN";
      }
    ).coerceAppLanguage;
    expect(coerce, "store must export coerceAppLanguage").toBeTypeOf(
      "function",
    );
    if (!coerce) return;

    expect(coerce("en")).toBe("en");
    expect(coerce("zh-CN")).toBe("zh-CN");
    expect(coerce("zh-TW")).toBe("en");
    expect(coerce(null)).toBe("en");
  });

  it("only requests a restart when the selected language changes", () => {
    const shouldRestart = (
      store as unknown as {
        shouldRestartForLanguageChange?: (
          current: "en" | "zh-CN",
          next: "en" | "zh-CN",
        ) => boolean;
      }
    ).shouldRestartForLanguageChange;
    expect(shouldRestart, "store must export language restart decision").toBeTypeOf(
      "function",
    );
    if (!shouldRestart) return;

    expect(shouldRestart("en", "zh-CN")).toBe(true);
    expect(shouldRestart("zh-CN", "en")).toBe(true);
    expect(shouldRestart("zh-CN", "zh-CN")).toBe(false);
  });
});
