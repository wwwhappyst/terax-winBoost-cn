import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect } from "react";

/**
 * 拦截页面内 http(s) 链接的默认跳转：改用系统浏览器打开。
 * 否则 WebView「确定导航」会离开应用源，界面表现为卡死。
 */
export function useExternalLinkGuard(): void {
  useEffect(() => {
    const onClick = (event: MouseEvent) => {
      if (event.defaultPrevented || event.button !== 0) return;
      // 不放行 Ctrl/Cmd/Shift/Alt + 点击：浏览器里这些组合是「新标签页/新窗口」，
      // 在 WebView2 中新窗口请求会被拦截且不会完成，页面随即整体失去响应。
      // 无论按没按修饰键，一律拦下改用系统浏览器打开。
      const target = event.target;
      if (!(target instanceof Element)) return;
      const anchor = target.closest("a[href]");
      if (!(anchor instanceof HTMLAnchorElement)) return;
      const href = anchor.getAttribute("href")?.trim();
      if (!href || href.startsWith("#") || href.startsWith("javascript:")) return;
      if (!/^https?:\/\//i.test(href)) return;
      event.preventDefault();
      event.stopPropagation();
      void openUrl(href).catch((err) => {
        console.error("[terax] openUrl failed:", err);
      });
    };
    document.addEventListener("click", onClick, true);
    return () => document.removeEventListener("click", onClick, true);
  }, []);
}
