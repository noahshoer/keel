import { describe, expect, it } from "vitest";
import { monthAt, monthTops, monthWindow } from "./calendarScroll";

describe("the calendar's scroll", () => {
  // Rows of 40 with a 12 gap between months.
  const tops = monthTops([5, 4, 6], 40, 12);

  it("stacks months by their rows", () => {
    expect(tops).toEqual([0, 212, 384]);
  });

  it("the month at the top is the shown one, from its first row", () => {
    expect(monthAt(tops, 0)).toBe(0);
    expect(monthAt(tops, 211)).toBe(0);
    expect(monthAt(tops, 212)).toBe(1);
    expect(monthAt(tops, 5000)).toBe(2);
  });

  it("settling a fraction of a pixel short still counts as that month", () => {
    expect(monthAt(tops, 211.6)).toBe(1);
  });

  it("draws a few months each side, inside the range", () => {
    expect(monthWindow(5, 144)).toEqual([2, 8]);
    expect(monthWindow(0, 144)).toEqual([0, 3]);
    expect(monthWindow(143, 144)).toEqual([140, 143]);
  });
});
