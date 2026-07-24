import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect } from "react";
import { usePreferencesStore } from "@/modules/settings/preferences";

/** 悬停多久后才抢焦点，避免掠过窗口时误聚焦。 */
const HOVER_FOCUS_DELAY_MS = 800;
/** 两次抢焦点最短间隔，避免任务输出/钩子回退导致焦点抖动、点击滚轮失效。 */
const FOCUS_COOLDOWN_MS = 2500;

/**
 * 开启后：指针进入失焦窗口并停留约 0.8 秒再 setFocus。
 * 用 Tauri 窗口焦点状态判断，不用 document.hasFocus()（WebView 上常误判，
 * 会导致反复抢焦点，点击/滚动失效）。
 */
export function useFocusFollowsMouse(): void {
  const enabled = usePreferencesStore((s) => s.focusFollowsMouse);

  useEffect(() => {
    if (!enabled) return;

    let timer: ReturnType<typeof setTimeout> | null = null;
    let pending = false;
    let windowFocused = true;
    let lastFocusAttemptAt = 0;
    let pointerDown = false;
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    const win = getCurrentWindow();

    const clearTimer = () => {
      if (timer !== null) {
        clearTimeout(timer);
        timer = null;
      }
    };

    const onPointerDown = () => {
      pointerDown = true;
      clearTimer();
    };
    const onPointerUp = () => {
      pointerDown = false;
    };

    const scheduleFocus = () => {
      if (windowFocused || pending || timer !== null || pointerDown) return;
      timer = setTimeout(() => {
        timer = null;
        void (async () => {
          if (windowFocused || pending || cancelled || pointerDown) return;
          const now = Date.now();
          if (now - lastFocusAttemptAt < FOCUS_COOLDOWN_MS) return;
          try {
            if (await win.isFocused()) {
              windowFocused = true;
              return;
            }
          } catch {
            // isFocused 失败时仍允许尝试一次 setFocus。
          }
          pending = true;
          lastFocusAttemptAt = now;
          try {
            await win.setFocus();
          } catch {
            // 忽略抢焦点失败。
          } finally {
            pending = false;
          }
        })();
      }, HOVER_FOCUS_DELAY_MS);
    };

    const onLeave = () => {
      clearTimer();
    };

    void win
      .isFocused()
      .then((focused) => {
        if (!cancelled) windowFocused = focused;
      })
      .catch(() => {});

    void win
      .onFocusChanged(({ payload }) => {
        windowFocused = payload;
        if (payload) clearTimer();
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {});

    // 仅在进入窗口时计时；不要在 mousemove 上反复调度。
    document.documentElement.addEventListener("mouseenter", scheduleFocus);
    document.documentElement.addEventListener("mouseleave", onLeave);
    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("pointerup", onPointerUp, true);
    window.addEventListener("pointercancel", onPointerUp, true);

    return () => {
      cancelled = true;
      clearTimer();
      unlisten?.();
      document.documentElement.removeEventListener("mouseenter", scheduleFocus);
      document.documentElement.removeEventListener("mouseleave", onLeave);
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("pointerup", onPointerUp, true);
      window.removeEventListener("pointercancel", onPointerUp, true);
    };
  }, [enabled]);
}
