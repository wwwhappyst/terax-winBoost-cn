// 验证新增 CLI 在通知界面使用稳定的产品显示名。
import { describe, expect, it } from "vitest";
import { displayAgent } from "./format";

describe("displayAgent", () => {
  it("formats Grok and OpenCode product names", () => {
    expect(displayAgent("grok")).toBe("Grok");
    expect(displayAgent("opencode")).toBe("OpenCode");
  });
});
