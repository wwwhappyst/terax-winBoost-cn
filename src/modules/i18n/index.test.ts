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

  it("translates agent hook errors with a product name", async () => {
    const module = await loadI18n();
    expect(module?.translate, "i18n module must export translate").toBeTypeOf(
      "function",
    );
    if (!module?.translate) return;

    expect(
      module.translate("zh-CN", "Failed to enable {agent} hooks", {
        agent: "OpenCode",
      }),
    ).toBe("启用 OpenCode Hook 失败");
  });

  it("translates common settings and action labels", async () => {
    const module = await loadI18n();
    expect(module?.translate, "i18n module must export translate").toBeTypeOf(
      "function",
    );
    if (!module?.translate) return;

    const cases: Array<[string, string]> = [
      ["General", "常规"],
      ["Editor", "编辑器"],
      ["Themes", "主题"],
      ["Shortcuts", "快捷键"],
      ["Models", "模型"],
      ["Agents", "智能体"],
      ["About", "关于"],
      ["Appearance", "外观"],
      ["Explorer", "资源管理器"],
      ["Terminal", "终端"],
      ["Startup", "启动"],
      ["Notifications", "通知"],
      ["Enable", "启用"],
      ["Enabling", "正在启用"],
      ["Clear", "清除"],
      ["Close", "关闭"],
      ["Cancel", "取消"],
      ["Delete", "删除"],
      ["Edit", "编辑"],
      ["Create", "创建"],
      ["Retry", "重试"],
      ["Loading…", "正在加载…"],
    ];
    for (const [english, chinese] of cases) {
      expect(module.translate("zh-CN", english), english).toBe(chinese);
    }
  });

  it("translates built-in agent descriptions and source-control guidance", async () => {
    const module = await loadI18n();
    expect(module?.translate, "i18n module must export translate").toBeTypeOf(
      "function",
    );
    if (!module?.translate) return;

    expect(
      module.translate(
        "zh-CN",
        "General-purpose coding assistant. Writes, edits, and runs.",
      ),
    ).toBe("通用编程助手，可编写、编辑和运行代码。");
    expect(
      module.translate(
        "zh-CN",
        "The active workspace is not inside a Git repository.",
      ),
    ).toBe("当前工作区不在 Git 仓库中。");
    expect(module.translate("zh-CN", "Use agent")).toBe("使用智能体");
    expect(module.translate("zh-CN", "AI chat")).toBe("智能体对话");
  });
});
