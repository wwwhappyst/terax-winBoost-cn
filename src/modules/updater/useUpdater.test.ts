// 验证上游版本比较只在正式版本更高时触发改版提醒。
import { describe, expect, it } from "vitest";
import { isNewerUpstreamVersion } from "./useUpdater";

describe("isNewerUpstreamVersion", () => {
  it("只接受比当前版本更高的上游版本", () => {
    expect(isNewerUpstreamVersion("0.8.6", "0.8.5")).toBe(true);
    expect(isNewerUpstreamVersion("v0.8.5", "0.8.5")).toBe(false);
    expect(isNewerUpstreamVersion("0.8.4", "0.8.5")).toBe(false);
  });
});
