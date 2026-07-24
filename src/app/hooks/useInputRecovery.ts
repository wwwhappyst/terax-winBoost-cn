import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect } from "react";
import { consumePendingAgentActivate } from "@/modules/agents/lib/pendingActivate";

/**
 * 窗口失焦/重获焦点时恢复交互：
 * - 清掉拖拽残留的 userSelect
 * - 仅在点系统通知拉回（有 pending 激活）时补 setFocus，并切到对应页签
 */
export function useInputRecovery(): void {
  useEffect(() => {
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    let wasFocused = true;

    const clearGestureResidue = () => {
      document.body.style.userSelect = "";
      document.body.style.cursor = "";
    };

    const onBlur = () => {
      clearGestureResidue();
    };

    const onVisibility = () => {
      if (document.visibilityState === "hidden") clearGestureResidue();
    };

    const onPointerCancel = () => {
      clearGestureResidue();
    };

    void win
      .isFocused()
      .then((focused) => {
        if (!cancelled) wasFocused = focused;
      })
      .catch(() => {});

    void win
      .onFocusChanged(({ payload: focused }) => {
        if (focused && !wasFocused) {
          clearGestureResidue();
          // 仅在「失焦系统通知 → 点回来」路径补 setFocus，避免普通 Alt-Tab 也抢焦点抖动。
          if (consumePendingAgentActivate()) {
            void win.setFocus().catch(() => {});
            requestAnimationFrame(() => {
              void win.setFocus().catch(() => {});
            });
          }
        } else if (!focused) {
          clearGestureResidue();
        }
        wasFocused = focused;
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {});

    window.addEventListener("blur", onBlur);
    document.addEventListener("visibilitychange", onVisibility);
    window.addEventListener("pointercancel", onPointerCancel, true);

    return () => {
      cancelled = true;
      unlisten?.();
      window.removeEventListener("blur", onBlur);
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("pointercancel", onPointerCancel, true);
    };
  }, []);
}
