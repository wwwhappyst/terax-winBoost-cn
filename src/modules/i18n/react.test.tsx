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

  it("translates string children and preserves React elements", () => {
    const translateNode = (
      i18n as unknown as {
        translateNode?: (value: React.ReactNode) => React.ReactNode;
      }
    ).translateNode;
    expect(translateNode, "i18n must export translateNode").toBeTypeOf(
      "function",
    );
    if (!translateNode) return;

    usePreferencesStore.setState({ language: "zh-CN" });
    expect(translateNode("Open settings")).toBe("打开设置");
    const icon = <span aria-hidden="true" />;
    expect(translateNode(icon)).toBe(icon);
  });

  it("translates text nested below an icon or wrapper", () => {
    const translateNode = (
      i18n as unknown as {
        translateNode?: (value: React.ReactNode) => React.ReactNode;
      }
    ).translateNode;
    expect(translateNode, "i18n must export translateNode").toBeTypeOf(
      "function",
    );
    if (!translateNode) return;

    usePreferencesStore.setState({ language: "zh-CN" });
    const node = (
      <span>
        <i aria-hidden="true" />
        <strong>Open settings</strong>
      </span>
    );
    expect(renderToStaticMarkup(<>{translateNode(node)}</>)).toContain(
      "打开设置",
    );
  });
});
