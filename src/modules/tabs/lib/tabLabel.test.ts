import { describe, expect, it } from "vitest";
import { labelFor, terminalTabNumbers } from "./tabLabel";
import type { EditorTab, TerminalTab } from "./useTabs";

function terminalTab(over: Partial<TerminalTab> = {}): TerminalTab {
  return {
    id: 1,
    kind: "terminal",
    spaceId: "default",
    title: "shell",
    paneTree: { kind: "leaf", id: 2 },
    activeLeafId: 2,
    ...over,
  };
}

function editorTab(id: number): EditorTab {
  return {
    id,
    kind: "editor",
    spaceId: "default",
    title: "a.ts",
    path: "/a.ts",
    dirty: false,
    preview: false,
  };
}

describe("terminalTabNumbers", () => {
  it("按栏内顺序给终端页签编号，跳过非终端", () => {
    const tabs = [
      terminalTab({ id: 10 }),
      editorTab(20),
      terminalTab({ id: 30 }),
    ];
    expect([...terminalTabNumbers(tabs).entries()]).toEqual([
      [10, 1],
      [30, 2],
    ]);
  });
});

describe("labelFor (terminal tabs)", () => {
  it("默认与上游一致：用 cwd 末段", () => {
    expect(labelFor(terminalTab({ cwd: "/Users/me/projects/terax-ai" }))).toBe(
      "terax-ai",
    );
    expect(labelFor(terminalTab({ cwd: "C:\\Users\\me\\proj" }))).toBe("proj");
  });

  it("开启 numberedLabels 时显示 tabN", () => {
    expect(
      labelFor(terminalTab({ cwd: "/Users/me/projects/terax-ai" }), {
        numberedLabels: true,
        terminalNumber: 1,
      }),
    ).toBe("tab1");
    expect(
      labelFor(terminalTab({ cwd: "C:\\Users\\me\\proj" }), {
        numberedLabels: true,
        terminalNumber: 2,
      }),
    ).toBe("tab2");
  });

  it("无 cwd 时回退到存储的 title", () => {
    expect(labelFor(terminalTab({ title: "private" }))).toBe("private");
  });

  it("优先使用自定义标题（含 numbered 模式）", () => {
    expect(
      labelFor(
        terminalTab({
          cwd: "/Users/me/projects/terax-ai",
          customTitle: "Server",
        }),
        { numberedLabels: true, terminalNumber: 1 },
      ),
    ).toBe("Server");
  });

  it("自定义标题在 cd 后仍保留", () => {
    const renamed = terminalTab({ cwd: "/Users/me/a", customTitle: "Server" });
    const afterCd = { ...renamed, cwd: "/Users/me/b/c" };
    expect(labelFor(afterCd)).toBe("Server");
  });
});
