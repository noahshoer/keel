// What iOS shows past the page's end matches the list's background (`--page`).
// When iOS scrolls the whole web view up to reveal a field under the keyboard,
// that space shows under the keyboard's rounded top corners; the web view's own
// color there is black. The page tells Swift its color (keel-ios
// `UnderPage.swift`) at start, whenever the theme is applied, and when the
// system's appearance changes (the "system" theme follows it).

type Handlers = Record<string, { postMessage: (m: unknown) => void }>;

export function installUnderPage(doc: Document, win: Window): { sync: () => void; uninstall: () => void } {
  let sent = "";
  const sync = () => {
    const handler = (win as { webkit?: { messageHandlers?: Handlers } }).webkit?.messageHandlers?.[
      "keelUnderPage"
    ];
    if (!handler) return;
    const color = win.getComputedStyle(doc.documentElement).getPropertyValue("--page").trim();
    if (!color || color === sent) return;
    sent = color;
    handler.postMessage(color);
  };
  const scheme = win.matchMedia?.("(prefers-color-scheme: dark)");
  scheme?.addEventListener("change", sync);
  sync();
  return { sync, uninstall: () => scheme?.removeEventListener("change", sync) };
}
