import type { Terminal } from "@xterm/xterm";

/**
 * Ink/Kimi 等 AI TUI 用反色单元格画视觉 caret，而 xterm 把 IME 锚在硬件光标上，
 * 两者常不一致（候选框漂到输入框右侧）。合成期间若扫到孤立反色格，则把
 * helper textarea / composition-view 钉到该格；否则回退到 xterm 默认行为。
 *
 * 思路来自社区 xterm-ime-anchor，内联以免新增依赖。
 */

export type ImeAnchorHandle = { detach(): void };

type Pin = { left: string; top: string };

function cellSize(terminal: Terminal, screen: HTMLElement): { w: number; h: number } {
  const rect = screen.getBoundingClientRect();
  return {
    w: rect.width / Math.max(terminal.cols, 1),
    h: rect.height / Math.max(terminal.rows, 1),
  };
}

/** 自右下向左上扫描孤立 inverse 单元格（Ink caret）；选区整行反色会被跳过。 */
export function findIsolatedInverseCell(
  terminal: Terminal,
): { col: number; row: number } | null {
  const buf = terminal.buffer.active;
  const rows = terminal.rows;
  const startY = buf.viewportY;

  for (let y = startY + rows - 1; y >= startY; y--) {
    const line = buf.getLine(y);
    if (!line) continue;
    for (let x = line.length - 1; x >= 0; x--) {
      const cell = line.getCell(x);
      if (!cell || !cell.isInverse()) continue;

      const left = x > 0 ? line.getCell(x - 1) : null;
      const right = x + 1 < line.length ? line.getCell(x + 1) : null;
      const leftInv = !!left?.isInverse();
      const rightInv = !!right?.isInverse();
      // 只要邻格反色，就视为选区/高亮条的一部分，而非单格 Ink caret。
      if (leftInv || rightInv) continue;

      return { col: x, row: y - startY };
    }
  }
  return null;
}

export function attachImeAnchor(terminal: Terminal): ImeAnchorHandle {
  const root = terminal.element;
  if (!root) return { detach() {} };

  const textarea = root.querySelector(
    ".xterm-helper-textarea",
  ) as HTMLTextAreaElement | null;
  const screen = root.querySelector(".xterm-screen") as HTMLElement | null;
  const compositionView = root.querySelector(
    ".composition-view",
  ) as HTMLElement | null;

  if (!textarea || !screen || !compositionView) {
    return { detach() {} };
  }

  let composing = false;
  let pinned: Pin | null = null;
  let renderDisposable: { dispose(): void } | null = null;

  const applyPin = (el: HTMLElement, pin: Pin) => {
    el.style.setProperty("left", pin.left, "important");
    el.style.setProperty("top", pin.top, "important");
  };

  const reapply = (el: HTMLElement) => {
    if (!composing || !pinned) return;
    if (el.style.left !== pinned.left || el.style.top !== pinned.top) {
      applyPin(el, pinned);
    }
  };

  const moTa = new MutationObserver(() => reapply(textarea));
  const moCv = new MutationObserver(() => reapply(compositionView));

  const recomputeAndPin = () => {
    if (!composing) return;
    const hit = findIsolatedInverseCell(terminal);
    if (!hit) return;

    const { w, h } = cellSize(terminal, screen);
    const left = `${Math.round(hit.col * w)}px`;
    const top = `${Math.round(hit.row * h)}px`;
    if (pinned && pinned.left === left && pinned.top === top) return;

    pinned = { left, top };
    applyPin(textarea, pinned);
    applyPin(compositionView, pinned);
  };

  const onCompositionStart = () => {
    composing = true;
    const hit = findIsolatedInverseCell(terminal);
    if (!hit) {
      pinned = null;
    } else {
      const { w, h } = cellSize(terminal, screen);
      pinned = {
        left: `${Math.round(hit.col * w)}px`,
        top: `${Math.round(hit.row * h)}px`,
      };
      applyPin(textarea, pinned);
      applyPin(compositionView, pinned);
    }
    renderDisposable = terminal.onRender(() => recomputeAndPin());
  };

  const onCompositionEnd = () => {
    composing = false;
    pinned = null;
    renderDisposable?.dispose();
    renderDisposable = null;
  };

  textarea.addEventListener("compositionstart", onCompositionStart);
  textarea.addEventListener("compositionend", onCompositionEnd);
  moTa.observe(textarea, { attributes: true, attributeFilter: ["style"] });
  moCv.observe(compositionView, {
    attributes: true,
    attributeFilter: ["style"],
  });

  return {
    detach() {
      composing = false;
      pinned = null;
      renderDisposable?.dispose();
      renderDisposable = null;
      textarea.removeEventListener("compositionstart", onCompositionStart);
      textarea.removeEventListener("compositionend", onCompositionEnd);
      moTa.disconnect();
      moCv.disconnect();
    },
  };
}
