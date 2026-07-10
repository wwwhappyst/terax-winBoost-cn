// 验证 React 组件与命令式调用都读取当前语言偏好。
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import { usePreferencesStore } from "@/modules/settings/preferences";
import * as i18n from "./index";

describe("translation state bindings", () => {
  afterEach(() => usePreferencesStore.setState({ language: "en" }));

  it("translates imperative messages using the current language", () => {
    const translateCurrent = (
      i18n as unknown as { t?: (text: string) => string }
    ).t;
    expect(translateCurrent, "i18n must export t").toBeTypeOf("function");
    if (!translateCurrent) return;

    usePreferencesStore.setState({ language: "zh-CN" });
    expect(translateCurrent("Open settings")).toBe("打开设置");
  });

  it("provides the translation function to React components", () => {
    const useTranslation = (
      i18n as unknown as {
        useTranslation?: () => (text: string) => string;
      }
    ).useTranslation;
    expect(useTranslation, "i18n must export useTranslation").toBeTypeOf(
      "function",
    );
    if (!useTranslation) return;

    const Probe = () => <span>{useTranslation()("Open settings")}</span>;
    expect(renderToStaticMarkup(<Probe />)).toContain("Open settings");
  });
});
