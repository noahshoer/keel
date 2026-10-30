// "/" (when not typing) or Ctrl/Cmd+K opens Search (crates/ui/src/main.rs
// `SEARCH_SHORTCUT_JS`).

export function isSearchShortcut(ev: KeyboardEvent, typing: boolean): boolean {
  const slash = ev.key === "/" && !typing && !ev.ctrlKey && !ev.metaKey && !ev.altKey;
  const ctrlK = (ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "k";
  return slash || ctrlK;
}

/** Whether focus is in a text field (where "/" is typed, not a shortcut). */
export function isTyping(el: Element | null): boolean {
  return (
    el instanceof HTMLInputElement ||
    el instanceof HTMLTextAreaElement ||
    (el instanceof HTMLElement && el.isContentEditable)
  );
}
