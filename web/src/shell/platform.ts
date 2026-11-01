import { useEffect, useState } from "react";
// Which kind of device the UI runs on, for text that names keys
// (crates/ui/src/platform.rs).
export type Platform = "touch" | "mac" | "other";

/** Classifies from `navigator.platform`, `navigator.maxTouchPoints` and the user agent. */
export function detect(platform: string, touchPoints: number, userAgent: string): Platform {
  const mobile = ["iPhone", "iPad", "iPod", "Android"];
  if (mobile.some((m) => platform.includes(m) || userAgent.includes(m))) return "touch";
  // iPadOS reports a Mac but has touch points; a Mac has none.
  if (platform.startsWith("Mac")) return touchPoints > 1 ? "touch" : "mac";
  return "other";
}

let current: Platform | null = null;

/** This device's platform (read from the navigator once). */
export function currentPlatform(): Platform {
  current ??= detect(navigator.platform ?? "", navigator.maxTouchPoints ?? 0, navigator.userAgent ?? "");
  return current;
}

/** The search page's hint: the shortcut only where there are keys. */
export function searchHint(p: Platform): string {
  if (p === "touch") return "Every word must match, in any order.";
  if (p === "mac") return "Every word must match, in any order. Press / or ⌘K anytime to search.";
  return "Every word must match, in any order. Press / or Ctrl+K anytime to search.";
}

/** The search row's tooltip. */
export function searchTooltip(p: Platform): string {
  if (p === "touch") return "Search";
  if (p === "mac") return "Search (/ or ⌘K)";
  return "Search (/ or Ctrl+K)";
}

const WIDE = "(min-width: 900px)";

/** Whether the window has the desktop layout, with the sidebar always showing. */
export function useWideLayout(): boolean {
  const [wide, setWide] = useState(() => typeof matchMedia === "function" && matchMedia(WIDE).matches);
  useEffect(() => {
    if (typeof matchMedia !== "function") return;
    const media = matchMedia(WIDE);
    const update = () => setWide(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);
  return wide;
}
