import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { usePreferencesStore } from "@/modules/settings/preferences";
import { labelFor, terminalTabNumbers } from "./tabLabel";
import type { Tab } from "./useTabs";

const APP_NAME = "Terax";

function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length ? parts[parts.length - 1]! : "/";
}

/**
 * Drives the OS window title from the focused tab + project folder, the way
 * Spotify shows the current track instead of just the app name. Without this
 * the window keeps the build-time default ("Tauri App" on Linux).
 *
 * Format: `<project> — <tab>`，页签名遵循 labelFor（默认 cwd，可选 tabN）。
 */
export function useWindowTitle(
  activeTab: Tab | undefined,
  explorerRoot: string | null,
  spaceTabs: Tab[] = [],
): void {
  const project = explorerRoot ? basename(explorerRoot) : "";
  const numberedLabels = usePreferencesStore(
    (s) => s.terminalNumberedTabLabels,
  );
  const numbers = terminalTabNumbers(spaceTabs);
  const label = activeTab
    ? labelFor(activeTab, {
        numberedLabels,
        terminalNumber: numbers.get(activeTab.id),
      })
    : "";

  useEffect(() => {
    let title: string;
    if (project && label && label !== project) title = `${project} — ${label}`;
    else title = project || label || APP_NAME;

    document.title = title;
    void getCurrentWindow()
      .setTitle(title)
      .catch(() => {});
  }, [project, label]);
}
