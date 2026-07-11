// 集中判断资源管理器的 Windows 重命名快捷键，避免误拦截组合键。
type RenameKeyEvent = Pick<
  KeyboardEvent,
  "key" | "altKey" | "ctrlKey" | "metaKey" | "shiftKey"
>;

/** 判断事件是否为无修饰键的 F2 重命名请求。 */
export function isExplorerRenameShortcut(event: RenameKeyEvent): boolean {
  return (
    event.key === "F2" &&
    !event.altKey &&
    !event.ctrlKey &&
    !event.metaKey &&
    !event.shiftKey
  );
}
