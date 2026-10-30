// Theme preference: "system" (follow the OS), "light" or "dark", applied as
// `data-theme` on <html> and remembered in localStorage (crates/ui/src/theme.rs).
export type Theme = "system" | "light" | "dark";

export function savedTheme(): Theme {
  try {
    const t = localStorage.getItem("theme");
    return t === "light" || t === "dark" ? t : "system";
  } catch {
    return "system";
  }
}

/** Applies and remembers a preference. */
export function applyTheme(t: Theme): void {
  try {
    localStorage.setItem("theme", t);
  } catch {
    // Private mode: the theme still applies for this session.
  }
  if (t === "system") delete document.documentElement.dataset["theme"];
  else document.documentElement.dataset["theme"] = t;
}

/** System → Light → Dark → System. */
export function nextTheme(t: Theme): Theme {
  return t === "system" ? "light" : t === "light" ? "dark" : "system";
}

export function themeLabel(t: Theme): string {
  if (t === "light") return "Theme: Light (click for Dark)";
  if (t === "dark") return "Theme: Dark (click for System)";
  return "Theme: System (click for Light)";
}
