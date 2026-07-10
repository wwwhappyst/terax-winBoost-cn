// 提供无依赖的界面翻译查询，并在缺少中文时安全回退英文。
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  Children,
  cloneElement,
  isValidElement,
  type ReactNode,
  useCallback,
} from "react";
import { ZH_CN } from "./zh-CN";

export type AppLanguage = "en" | "zh-CN";
export type TranslationParams = Record<string, string | number>;

/** 查询界面翻译，并替换调用方传入的动态参数。 */
export function translate(
  language: AppLanguage,
  text: string,
  params: TranslationParams = {},
): string {
  const template = language === "zh-CN" ? (ZH_CN[text] ?? text) : text;
  return template.replace(/\{(\w+)\}/g, (match, key: string) =>
    Object.prototype.hasOwnProperty.call(params, key)
      ? String(params[key])
      : match,
  );
}

/** 为 React 组件提供随语言偏好更新的翻译函数。 */
export function useTranslation() {
  const language = usePreferencesStore((state) => state.language);
  return useCallback(
    (text: string, params?: TranslationParams) =>
      translate(language, text, params),
    [language],
  );
}

/** 为 Toast 等命令式调用读取当前语言偏好。 */
export function t(text: string, params?: TranslationParams): string {
  return translate(usePreferencesStore.getState().language, text, params);
}

/** 翻译组件边界上的字符串 children，事件和其他属性保持原样。 */
export function translateNode(value: ReactNode): ReactNode {
  if (typeof value === "string") return t(value);
  if (Array.isArray(value)) return Children.map(value, translateNode);
  if (
    isValidElement<{ children?: ReactNode }>(value) &&
    value.props.children !== undefined
  ) {
    return cloneElement(value, {}, translateNode(value.props.children));
  }
  return value;
}
