// 在顶栏提示原作者有新版本，并引导用户同步 Fork，不提供直接安装。
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { t } from "@/modules/i18n";
import { SystemUpdate01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import { useUpdater } from "./useUpdater";

export function UpdaterDialog() {
  const { status } = useUpdater();
  const [open, setOpen] = useState(false);
  if (status.kind !== "manual-available") return null;

  return (
    <>
      <Button
        variant="ghost"
        size="icon"
        className="relative size-7 shrink-0 rounded-md text-primary hover:bg-accent hover:text-primary"
        title={t("Upstream update available")}
        onClick={() => setOpen(true)}
      >
        <HugeiconsIcon icon={SystemUpdate01Icon} size={16} strokeWidth={1.75} />
        <span className="absolute -top-0.5 -right-0.5 size-2 rounded-full bg-destructive" />
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-[440px]">
          <DialogHeader>
            <DialogTitle>
              {t("Upstream Terax v{version} is available", {
                version: status.info.version,
              })}
            </DialogTitle>
            <DialogDescription>
              {t(
                "This Windows edition does not install upstream packages directly. Sync your Fork, update main, then merge it into the Windows branch.",
              )}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="ghost" size="sm" onClick={() => setOpen(false)}>
              {t("Later")}
            </Button>
            <Button
              size="sm"
              onClick={() => void openUrl(status.info.releaseUrl)}
            >
              {t("View upstream release")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
