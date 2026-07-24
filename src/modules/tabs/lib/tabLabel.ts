import type { Tab } from "./useTabs";

export type LabelForOptions = {
  /** 开启后，无自定义名的终端页签显示为 tabN。默认关闭以保持上游 cwd 命名。 */
  numberedLabels?: boolean;
  /** 当前空间内终端页签的 1-based 序号；仅 numberedLabels 时使用。 */
  terminalNumber?: number;
};

/**
 * 当前列表中终端页签的 1-based 序号（按栏内顺序）。
 * 非终端页签不计入，避免夹在中间的编辑器把序号撑出空洞。
 */
export function terminalTabNumbers(tabs: readonly Tab[]): Map<number, number> {
  const out = new Map<number, number>();
  let n = 0;
  for (const t of tabs) {
    if (t.kind === "terminal") out.set(t.id, ++n);
  }
  return out;
}

/**
 * 页签栏展示名。非终端用各自 title；终端优先自定义名。
 * 默认（与上游一致）回退到 cwd 末段；可选 numberedLabels 时用 tabN。
 */
export function labelFor(t: Tab, options?: LabelForOptions): string {
  if (t.kind === "editor") return t.title;
  if (t.kind === "preview") return t.title;
  if (t.kind === "markdown") return t.title;
  if (t.kind === "ai-diff") return t.title;
  if (t.kind === "git-diff") return t.title;
  if (t.kind === "git-history") return t.title;
  if (t.kind === "git-commit-file") return t.title;
  if (t.customTitle) return t.customTitle;
  if (
    options?.numberedLabels &&
    options.terminalNumber != null &&
    t.kind === "terminal"
  ) {
    return `tab${options.terminalNumber}`;
  }
  if (!t.cwd) return t.title;
  const parts = t.cwd.split(/[\\/]/).filter(Boolean);
  return parts.length ? parts[parts.length - 1]! : "/";
}
