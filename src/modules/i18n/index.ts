// 提供无依赖的界面翻译查询，并在缺少中文时安全回退英文。
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
    Object.hasOwn(params, key) ? String(params[key]) : match,
  );
}
