// Text must reach 4.5:1 against every surface it sits on, in both themes.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { LABEL_COLORS } from "../editor/labelColors";

// From disk: Vitest empties CSS imports, `?raw` included.
const css = readFileSync(resolve(process.cwd(), "src/styles/app.css"), "utf8");

type Tokens = Record<string, string>;

function block(start: string): Tokens {
  const from = css.indexOf(start);
  if (from < 0) throw new Error(`no block ${start}`);
  const open = css.indexOf("{", from + start.length - 1);
  const body = css.slice(open + 1, css.indexOf("\n}", open));
  const tokens: Tokens = {};
  for (const m of body.matchAll(/(--[a-z0-9-]+):\s*(#[0-9a-fA-F]{3,8})\s*;/g)) {
    const [, name, value] = m;
    if (name !== undefined && value !== undefined) tokens[name] = value;
  }
  return tokens;
}

function luminance(hex: string): number {
  const channel = (i: number) => {
    const c = Number.parseInt(hex.slice(1 + i * 2, 3 + i * 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(0) + 0.7152 * channel(1) + 0.0722 * channel(2);
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

const THEMES: Array<[string, Tokens]> = [
  ["light", block(":root {")],
  ["dark (system)", block(':root:not([data-theme="light"]) {')],
  ["dark (chosen)", block(':root[data-theme="dark"] {')],
];

const TEXT = [
  "--text",
  "--secondary",
  "--subtle",
  "--muted",
  "--due-overdue",
  "--due-today",
  "--due-tomorrow",
  "--due-week",
  "--due-later",
  "--p1",
  "--p2-text",
  "--p3-text",
  "--accent-text",
  "--streak",
];
const SURFACES = ["--bg", "--app-bg", "--page", "--page-wide", "--chrome", "--panel"];

/** `fg` at `share` over `bg`, as `color-mix(in srgb, fg share, bg)`. */
function mix(fg: string, share: number, bg: string): string {
  const channel = (hex: string, i: number) => Number.parseInt(hex.slice(1 + i * 2, 3 + i * 2), 16);
  const part = (i: number) =>
    Math.round(channel(fg, i) * share + channel(bg, i) * (1 - share))
      .toString(16)
      .padStart(2, "0");
  return `#${part(0)}${part(1)}${part(2)}`;
}

/** The colors a filled pill's text can be: the tint behind it is the same color. */
const PILL_TEXT = [
  "--accent-text",
  "--due-overdue",
  "--due-today",
  "--due-tomorrow",
  "--due-week",
  "--due-later",
  "--p1",
  "--p2-text",
  "--p3-text",
  ...LABEL_COLORS.map((c) => `--label-${c.id}`),
];

describe.each(THEMES)("a new name's pill, %s", (_theme, tokens) => {
  it("text on its fill is at least 4.5:1", () => {
    expect(tokens["--new-text"]).toBeDefined();
    expect(contrast(tokens["--new-text"] as string, tokens["--new-fill"] as string)).toBeGreaterThanOrEqual(
      4.5,
    );
  });
});

/** The token's color over `--bg`: hex as is, `rgba()` mixed in. */
function onBg(value: string, bg: string): string {
  const m = value.match(/^rgba\((\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\)$/);
  if (!m) return value;
  const hex = (n: number) => n.toString(16).padStart(2, "0");
  return mix(`#${hex(Number(m[1]))}${hex(Number(m[2]))}${hex(Number(m[3]))}`, Number(m[4]), bg);
}

describe.each(THEMES)("the filter and sort buttons and chips, %s", (_theme, tokens) => {
  it("a set button's icon on --active-tint is at least 4.5:1", () => {
    expect(css).toMatch(
      /\.arrange-btn\.on \{[^}]*background: var\(--active-tint\)[^}]*color: var\(--accent-text\)/,
    );
    // `block` reads hex only; the dark themes' tint is an rgba() (both dark blocks use the same one).
    const rgba = [...css.matchAll(/--active-tint:\s*(rgba\([^)]*\))/g)].pop()?.[1];
    const tint = onBg(tokens["--active-tint"] ?? (rgba as string), tokens["--bg"] as string);
    expect(contrast(tokens["--accent-text"] as string, tint)).toBeGreaterThanOrEqual(4.5);
  });
  it("a broken filter's chip text on its fill is at least 4.5:1", () => {
    expect(css).toMatch(
      /\.arrange-chip\.problem \{[^}]*background: color-mix\(in srgb, var\(--due-overdue\) 16%, var\(--bg\)\); color: var\(--due-overdue\)/,
    );
    const fg = tokens["--due-overdue"] as string;
    expect(contrast(fg, mix(fg, 0.16, tokens["--bg"] as string))).toBeGreaterThanOrEqual(4.5);
  });
});

describe.each(THEMES)("a filled pill, %s", (_theme, tokens) => {
  it.each(PILL_TEXT)("%s on its own tint is at least 4.5:1", (t) => {
    const fg = tokens[t] as string;
    expect(fg, t).toBeDefined();
    expect(contrast(fg, mix(fg, 0.16, tokens["--bg"] as string))).toBeGreaterThanOrEqual(4.5);
  });
});

it("a filled pill's tint is that 16%", () => {
  expect(css).toMatch(
    /\.chip-btn\.pill-set[^{]*\{[^}]*background: color-mix\(in srgb, currentColor 16%, var\(--bg\)\)/,
  );
});

describe.each(THEMES)("contrast, %s", (_theme, tokens) => {
  it.each(TEXT.flatMap((t) => SURFACES.map((s) => [t, s] as const)))("%s on %s is at least 4.5:1", (t, s) => {
    const fg = tokens[t];
    const bg = tokens[s];
    expect(fg, t).toBeDefined();
    expect(bg, s).toBeDefined();
    expect(contrast(fg as string, bg as string)).toBeGreaterThanOrEqual(4.5);
  });
});

describe.each(THEMES)("label colors, %s", (_theme, tokens) => {
  it.each(LABEL_COLORS.map((c) => c.id))("--label-%s is at least 4.5:1 on --bg", (id) => {
    const fg = tokens[`--label-${id}`];
    expect(fg).toBeDefined();
    expect(contrast(fg as string, tokens["--bg"] as string)).toBeGreaterThanOrEqual(4.5);
  });
});

describe.each(THEMES)("a day outside the shown month, %s", (_theme, tokens) => {
  it("is readable on --bg", () => {
    expect(contrast(tokens["--cal-other"] as string, tokens["--bg"] as string)).toBeGreaterThanOrEqual(4.5);
  });
  it("is grayer than other secondary text", () => {
    const toBg = (t: string) => contrast(tokens[t] as string, tokens["--bg"] as string);
    expect(toBg("--cal-other")).toBeLessThan(toBg("--secondary"));
  });
});

describe("the approved light values", () => {
  const light = block(":root {");
  it.each([
    ["--secondary", "#5a5a5a"],
    ["--due-today", "#1f6f55"],
    ["--due-overdue", "#b83328"],
    ["--p4", "#767676"],
    ["--icon", "#3d3d3d"],
  ])("%s is %s", (token, value) => {
    expect(light[token]).toBe(value);
  });
});

describe.each(THEMES)("a selected day, %s", (_theme, tokens) => {
  it.each(["--due-overdue", "--due-today", "--due-tomorrow", "--due-week", "--accent"])(
    "--on-fill is at least 4.5:1 on %s",
    (fill) => {
      expect(contrast(tokens["--on-fill"] as string, tokens[fill] as string)).toBeGreaterThanOrEqual(4.5);
    },
  );
});

describe("icons", () => {
  it.each(THEMES.slice(1))("are pure white, %s", (_theme, tokens) => {
    expect(tokens["--icon"]).toBe("#ffffff");
  });
});

describe("due date colors", () => {
  it.each(THEMES)("are told apart by when, %s", (_theme, tokens) => {
    const due = ["--due-overdue", "--due-today", "--due-tomorrow", "--due-week", "--due-later"];
    expect(new Set(due.map((t) => tokens[t])).size).toBe(due.length);
  });
});

function blend(fg: string, bg: string, share: number): string {
  const part = (h: string, i: number) => Number.parseInt(h.slice(1 + i * 2, 3 + i * 2), 16);
  const mix = (i: number) => Math.round(part(fg, i) * share + part(bg, i) * (1 - share));
  return `#${[0, 1, 2].map((i) => mix(i).toString(16).padStart(2, "0")).join("")}`;
}

describe.each(THEMES)("an open pill's tint, %s", (_theme, tokens) => {
  const share =
    Number(
      css.match(
        /\.pill-tinted\.chip-btn\.chip-open \{ background: color-mix\(in srgb, currentColor (\d+)%, var\(--bg\)\)/,
      )?.[1],
    ) / 100;
  it("is a share of the color over the card", () => {
    expect(share).toBeGreaterThan(0);
  });
  it.each(["--p1", "--p2-text", "--p3-text"])("keeps %s at least 4.5:1", (t) => {
    const fg = tokens[t] as string;
    expect(contrast(fg, blend(fg, tokens["--bg"] as string, share))).toBeGreaterThanOrEqual(4.5);
  });
});

describe.each(THEMES)("a chosen label's color on its tint, %s", (_theme, tokens) => {
  const bg = tokens["--bg"] as string;
  const row =
    Number(
      css.match(
        /\.label-row\.on\[style\*="color"\] \{ background: color-mix\(in srgb, currentColor (\d+)%/,
      )?.[1],
    ) / 100;
  const pill =
    Number(
      css.match(
        /\.pill-tinted\.chip-btn\.chip-open \{ background: color-mix\(in srgb, currentColor (\d+)%/,
      )?.[1],
    ) / 100;
  it.each(LABEL_COLORS.map((c) => c.id))("--label-%s is at least 4.5:1 on a row and a pill", (id) => {
    const fg = tokens[`--label-${id}`] as string;
    expect(contrast(fg, blend(fg, bg, row))).toBeGreaterThanOrEqual(4.5);
    expect(contrast(fg, blend(fg, bg, pill))).toBeGreaterThanOrEqual(4.5);
  });
});

describe("text that used to miss the rule", () => {
  it("every quick choice has its color, later dates in Keel green", () => {
    expect(css).toMatch(
      /\.quick-later \.menu-label, \.quick-later \.menu-hint \{ color: var\(--accent-text\); \}/,
    );
  });
  it("other-month days aren't faded below it", () => {
    expect(css).not.toMatch(/\.cal-day\.other \{[^}]*opacity/);
  });
  it("gray in the calendar only means outside the shown month, so a past month still lights up", () => {
    expect(css).not.toMatch(/\.cal-day\.past/);
  });
});

describe("quick date choices", () => {
  it.each([
    ["overdue", "--due-overdue"],
    ["today", "--due-today"],
    ["tomorrow", "--due-tomorrow"],
    ["week", "--due-week"],
    ["later", "--accent-text"],
  ])("%s colors its name and its day alike", (bucket, token) => {
    expect(css).toContain(
      `.quick-${bucket} .menu-label, .quick-${bucket} .menu-hint { color: var(${token}); }`,
    );
  });
});

describe("phone tap targets", () => {
  it.each([".search-clear", ".view-toggle", ".tv-toggle", ".btn"])("%s reaches 44pt", (sel) => {
    const touch = css.slice(css.indexOf("/* Tap targets reach 44pt"));
    expect(touch).toContain(`${sel}::after`);
  });
});

describe("the view header", () => {
  it("has a fade below it that grows with the list's scroll", () => {
    expect(css).toMatch(/\.view-head::after \{[^}]*linear-gradient\(\s*var\(--page-bg\) 14%,/);
    expect(css).toMatch(
      /\.view-head::after \{[^}]*transform-origin: top; transform: scaleY\(var\(--list-fade, 0\)\)/,
    );
  });

  it("is moved frame by frame from script, on its own layer, with no transitions to restart", () => {
    const phone = css.slice(css.indexOf("@media (max-width: 700px) and (pointer: coarse)"));
    expect(phone).toMatch(/\.view-head \.view-title \{[^}]*will-change: translate, scale;/);
    expect(phone).not.toMatch(/\.view-head[^{]*\{[^}]*transition:/);
    expect(css).not.toMatch(/\.view-head::after \{[^}]*transition:/);
  });

  it("doesn't use a scroll-linked animation, which iOS doesn't draw at rest", () => {
    expect(css).not.toContain("animation-timeline");
  });

  it("on the phone, shrinks over the list instead of resizing it", () => {
    const phone = css.slice(css.indexOf("@media (max-width: 700px) and (pointer: coarse)"));
    expect(phone).toMatch(/\.view-head \{[^}]*position: absolute;/);
    expect(phone).toMatch(/\.list-body \{ padding-top: var\(--head-h\); \}/);
    expect(phone).toMatch(
      /\.view-head \.view-title \{[^}]*scale: calc\(1 - var\(--title-move, 0\) \* 0\.412\)/,
    );
  });
});

describe("lists on touch", () => {
  it("always scroll, however short, so they bounce like a full list", () => {
    expect(css).toContain(
      "@media (pointer: coarse) { .list-scroll > .list-body { min-height: calc(100% + 1px); } }",
    );
  });
});

describe("calendar days on the phone", () => {
  it("reach 44pt beyond their drawn 40", () => {
    expect(css).toMatch(/\.cal-day::after \{ content: ''; position: absolute; inset: -2px; \}/);
  });
});

describe("radii", () => {
  it("has the card, panel and item radii", () => {
    expect(css).toMatch(/--r-card:\s*20px/);
    expect(css).toMatch(/--r-panel:\s*20px/);
    expect(css).toMatch(/--r-item:\s*12px/);
    expect(css).toMatch(/@media \(min-width: 900px\) \{ :root \{ --r-card: 16px; \} \}/);
  });
});

describe("a new-name pill with its picker open", () => {
  // Every rule whose selector names `.pill-new.chip-open`: selector and body.
  const bare = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const rules = [...bare.matchAll(/([^{}]*\.pill-new\.chip-open[^{}]*)\{([^}]*)\}/g)].map((m) => ({
    selector: m[1] ?? "",
    body: m[2] ?? "",
  }));

  it("turns its border solid and its text bold, in both cards", () => {
    const rule = rules.find(
      (r) => /border(-style)?:[^;]*solid/.test(r.body) && /font-weight:\s*600/.test(r.body),
    );
    expect(rule).toBeDefined();
    const selectors = (rule?.selector ?? "").split(",").map((s) => s.trim());
    expect(selectors).toContain(".chip-btn.pill-new.chip-open");
    expect(selectors).toContain(".on-keyboard .pill-tinted.chip-btn.pill-new.chip-open");
    expect(rule?.body).toContain("var(--new-");
  });
});
