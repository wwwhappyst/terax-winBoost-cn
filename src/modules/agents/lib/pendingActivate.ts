/** 失焦时发出的系统通知：等窗口被点回后执行一次页签激活。 */
type Pending = {
  at: number;
  activate: () => void;
};

let pending: Pending | null = null;

const MAX_AGE_MS = 120_000;

export function setPendingAgentActivate(activate: () => void): void {
  pending = { at: Date.now(), activate };
}

export function consumePendingAgentActivate(): boolean {
  if (!pending) return false;
  if (Date.now() - pending.at > MAX_AGE_MS) {
    pending = null;
    return false;
  }
  const { activate } = pending;
  pending = null;
  try {
    activate();
  } catch (err) {
    console.warn("[terax] pending agent activate failed:", err);
  }
  return true;
}

export function clearPendingAgentActivate(): void {
  pending = null;
}
