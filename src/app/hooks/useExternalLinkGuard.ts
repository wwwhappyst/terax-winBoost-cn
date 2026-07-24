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
      if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
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
