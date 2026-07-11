// 验证智能体浮窗缩放值保持在界面允许的安全范围内。
import { describe, expect, it } from "vitest";
import * as store from "./store";

describe("coerceAiMiniZoom", () => {
  it("clamps invalid values and preserves supported values", () => {
    const coerce = (
      store as unknown as {
        coerceAiMiniZoom?: (value: unknown) => number;
      }
    ).coerceAiMiniZoom;
    expect(coerce, "store must export AI mini zoom coercion").toBeTypeOf(
      "function",
    );
    if (!coerce) return;

    expect(coerce(1.25)).toBe(1.25);
    expect(coerce(0)).toBe(0.5);
    expect(coerce(9)).toBe(2);
    expect(coerce("bad")).toBe(1);
  });
});
