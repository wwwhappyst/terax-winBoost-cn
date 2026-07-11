// 只读取原作者 Release 版本，改版程序绝不下载或安装上游安装包。
import { getVersion } from "@tauri-apps/api/app";
import { useCallback, useEffect, useState } from "react";

const GITHUB_LATEST_RELEASE =
  "https://api.github.com/repos/crynta/terax-ai/releases/latest";

export interface ManualUpdateInfo {
  version: string;
  currentVersion: string;
  releaseUrl: string;
}

export type UpdaterStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "uptodate" }
  | { kind: "manual-available"; info: ManualUpdateInfo }
  | { kind: "error"; message: string };

function parseVersion(version: string): number[] {
  return version
    .replace(/^v/, "")
    .split("-")[0]
    .split(".")
    .map((part) => Number.parseInt(part, 10) || 0);
}

/** 判断原作者的版本是否高于当前改版版本。 */
export function isNewerUpstreamVersion(remote: string, current: string): boolean {
  const upstream = parseVersion(remote);
  const local = parseVersion(current);
  const length = Math.max(upstream.length, local.length);
  for (let index = 0; index < length; index++) {
    const remotePart = upstream[index] ?? 0;
    const currentPart = local[index] ?? 0;
    if (remotePart !== currentPart) return remotePart > currentPart;
  }
  return false;
}

async function checkUpstreamRelease(): Promise<ManualUpdateInfo | null> {
  const [currentVersion, response] = await Promise.all([
    getVersion(),
    fetch(GITHUB_LATEST_RELEASE, {
      headers: { Accept: "application/vnd.github+json" },
    }),
  ]);
  if (!response.ok) throw new Error(`GitHub API ${response.status}`);

  const release = (await response.json()) as {
    tag_name: string;
    html_url: string;
  };
  const version = release.tag_name.replace(/^v/, "");
  if (!isNewerUpstreamVersion(version, currentVersion)) return null;
  return { version, currentVersion, releaseUrl: release.html_url };
}

/** 应用启动时检查一次原作者 Release，并只返回人工同步提醒。 */
export function useUpdater() {
  const [status, setStatus] = useState<UpdaterStatus>({ kind: "idle" });

  const check = useCallback(async () => {
    setStatus({ kind: "checking" });
    try {
      const info = await checkUpstreamRelease();
      setStatus(info ? { kind: "manual-available", info } : { kind: "uptodate" });
    } catch (error) {
      // 网络失败不打断启动，仅保留开发日志供排查。
      console.warn("upstream release check failed", error);
      setStatus({ kind: "error", message: String(error) });
    }
  }, []);

  useEffect(() => {
    void check();
  }, [check]);

  return { status, check };
}
