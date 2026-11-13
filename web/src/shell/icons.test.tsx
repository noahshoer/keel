import { render } from "@testing-library/react";
import { createElement, type FunctionComponent } from "react";
import { describe, expect, it } from "vitest";
import * as icons from "./icons";

// Not Phosphor icons: Keel's own marks.
const OWN_MARKS = new Set(["Logo", "DoneMark"]);

const iconExports = Object.entries(icons).filter(([name]) => name.endsWith("Icon") && !OWN_MARKS.has(name));

// A stroke in the icon's own color makes these heavier than Phosphor's bold.
const HEAVIER = new Set([
  "MenuIcon",
  "SearchIcon",
  "SunIcon",
  "MoonIcon",
  "PlusIcon",
  "MoreIcon",
  "CheckCircleIcon",
]);

describe("icons", () => {
  it("has the row icons the task layout needs", () => {
    const names = iconExports.map(([name]) => name);
    for (const name of [
      "FireIcon",
      "FlagFillIcon",
      "BellRingingIcon",
      "TagIcon",
      "CircleDashedIcon",
      "CheckCircleIcon",
      "SortIcon",
    ]) {
      expect(names).toContain(name);
    }
  });

  it.each(iconExports)("%s is one filled svg inside span.icon", (name, icon) => {
    const props = { size: 18, day: 7 };
    const { container } = render(createElement(icon as FunctionComponent<typeof props>, props));
    const svgs = container.querySelectorAll("span.icon > svg");
    expect(svgs).toHaveLength(1);
    const svg = svgs[0] as SVGElement;
    expect(svg.getAttribute("fill")).toBe("currentColor");
    expect(svg.getAttribute("aria-hidden")).toBe("true");
    expect(svg.getAttribute("stroke")).toBe(HEAVIER.has(name) ? "currentColor" : null);
  });

  // Keel's own drawings, rounder than Phosphor's: a 24 line with round corners inside and out.
  const DRAWN = new Set([
    "TagIcon",
    "TagSmallIcon",
    "InboxIcon",
    "TodayIcon",
    "UpcomingIcon",
    "CalendarSmallIcon",
    "TrophySmallIcon",
  ]);
  const drawnExports = iconExports.filter(([name]) => DRAWN.has(name));

  it("draws all of them", () => {
    expect(drawnExports).toHaveLength(DRAWN.size);
  });

  it.each(drawnExports)("%s is drawn with a round 24 line", (name, icon) => {
    const { container } = render(createElement(icon as FunctionComponent<{ day: number }>, { day: 7 }));
    const line = container.querySelector('svg [stroke-width="24"]');
    expect(line?.getAttribute("stroke-linejoin")).toBe("round");
    if (!name.startsWith("Tag") && !name.startsWith("Trophy"))
      expect(container.querySelector("svg rect")?.getAttribute("rx")).toBe("20");
  });

  it("shows the day inside Today's calendar", () => {
    const { container } = render(<icons.TodayIcon day={7} />);
    expect(container.querySelector("svg text")?.textContent).toBe("7");
  });
});
