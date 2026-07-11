// 验证资源管理器只把无修饰键的 F2 视为 Windows 重命名快捷键。
import { describe, expect, it } from "vitest";
import * as shortcut from "./renameShortcut";

describe("isExplorerRenameShortcut", () => {
  it("accepts plain F2 only", () => {
    const isRename = (
      shortcut as unknown as {
        isExplorerRenameShortcut?: (event: {
          key: string;
          altKey: boolean;
          ctrlKey: boolean;
          metaKey: boolean;
          shiftKey: boolean;
        }) => boolean;
      }
    ).isExplorerRenameShortcut;
    expect(isRename, "rename shortcut helper must be exported").toBeTypeOf(
      "function",
    );
    if (!isRename) return;

    expect(
      isRename({
        key: "F2",
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isRename({
        key: "F2",
        altKey: false,
        ctrlKey: true,
        metaKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
  });
});
