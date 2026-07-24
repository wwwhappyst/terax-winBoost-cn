import { describe, expect, it } from "vitest";
import { findIsolatedInverseCell } from "./imeAnchor";

type FakeCell = { isInverse: () => boolean };

function lineOf(flags: boolean[]) {
  return {
    length: flags.length,
    getCell: (x: number): FakeCell | undefined =>
      x >= 0 && x < flags.length
        ? { isInverse: () => flags[x]! }
        : undefined,
  };
}

describe("findIsolatedInverseCell", () => {
  it("返回右下角孤立反色格（Ink caret）", () => {
    const terminal = {
      rows: 2,
      buffer: {
        active: {
          viewportY: 0,
          getLine: (y: number) => {
            if (y === 0) return lineOf([false, false, false]);
            if (y === 1) return lineOf([false, true, false]);
            return undefined;
          },
        },
      },
    };
    expect(findIsolatedInverseCell(terminal as never)).toEqual({
      col: 1,
      row: 1,
    });
  });

  it("跳过整段反色选区", () => {
    const terminal = {
      rows: 1,
      buffer: {
        active: {
          viewportY: 0,
          getLine: () => lineOf([true, true, true]),
        },
      },
    };
    expect(findIsolatedInverseCell(terminal as never)).toBeNull();
  });

  it("无反色时回退（返回 null，由调用方用硬件光标）", () => {
    const terminal = {
      rows: 1,
      buffer: {
        active: {
          viewportY: 0,
          getLine: () => lineOf([false, false]),
        },
      },
    };
    expect(findIsolatedInverseCell(terminal as never)).toBeNull();
  });
});
