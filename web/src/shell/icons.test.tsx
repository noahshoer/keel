import { render } from "@testing-library/react";
import { createElement, type FunctionComponent } from "react";
import { describe, expect, it } from "vitest";
import * as icons from "./icons";

// Not Phosphor icons: Keel's own marks.
const OWN_MARKS = new Set(["Logo", "DoneMark"]);

const iconExports = Object.entries(icons).filter(([name]) => name.endsWith("Icon") && !OWN_MARKS.has(name));

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
    ]) {
      expect(names).toContain(name);
    }
  });

  it.each(iconExports)("%s is one filled svg inside span.icon", (_name, icon) => {
    const props = { size: 18, day: 7 };
    const { container } = render(createElement(icon as FunctionComponent<typeof props>, props));
    const svgs = container.querySelectorAll("span.icon > svg");
    expect(svgs).toHaveLength(1);
    const svg = svgs[0] as SVGElement;
    expect(svg.getAttribute("fill")).toBe("currentColor");
    expect(svg.getAttribute("aria-hidden")).toBe("true");
    expect(svg.getAttribute("stroke")).toBeNull();
  });

  it("shows the day inside Today's calendar", () => {
    const { container } = render(<icons.TodayIcon day={7} />);
    expect(container.querySelector("svg text")?.textContent).toBe("7");
  });
});
