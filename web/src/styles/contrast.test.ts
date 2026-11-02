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
    expect(css).toMatch(/\.quick-later \.menu-label \{ color: var\(--accent-text\); \}/);
  });
  it("quick-add chips use the text colors", () => {
    expect(css).toMatch(/\.quick-chip\.chip-project \{ color: var\(--p3-text\); \}/);
    expect(css).toMatch(/\.quick-chip\.chip-label \{ color: var\(--p2-text\); \}/);
  });
  it("past and other-month days aren't faded below it", () => {
    expect(css).not.toMatch(/\.cal-day\.(past|other) \{[^}]*opacity/);
  });
});

describe("phone tap targets", () => {
  it.each([".search-clear", ".view-toggle", ".tv-toggle", ".btn"])("%s reaches 44pt", (sel) => {
    const touch = css.slice(css.indexOf("/* Tap targets reach 44pt"));
    expect(touch).toContain(`${sel}::after`);
  });
});

describe("calendar days on the phone", () => {
  it("reach 44pt beyond their drawn 40", () => {
    expect(css).toMatch(/\.on-keyboard \.cal-day::after \{ content: ''; position: absolute; inset: -2px; \}/);
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
