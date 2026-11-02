import { describe, expect, it } from "vitest";
import { monthAt, weekWindow } from "./calendarScroll";

describe("the calendar's scroll", () => {
  const tops = [0, 200, 360];

  it("the month at the top is the shown one, from the week of its 1st", () => {
    expect(monthAt(tops, 0)).toBe(0);
    expect(monthAt(tops, 199)).toBe(0);
    expect(monthAt(tops, 200)).toBe(1);
    expect(monthAt(tops, 5000)).toBe(2);
  });

  it("settling a fraction of a pixel short still counts as that month", () => {
    expect(monthAt(tops, 199.6)).toBe(1);
  });

  it("draws weeks well past the view each way, inside the range", () => {
    expect(weekWindow(0, 600)).toEqual([0, 24]);
    expect(weekWindow(40, 600)).toEqual([32, 64]);
    expect(weekWindow(599, 600)).toEqual([584, 600]);
  });

  it("keeps the same window while the top stays in one stretch of weeks, so it rarely redraws", () => {
    expect(weekWindow(40, 600)).toEqual(weekWindow(47, 600));
    expect(weekWindow(48, 600)).not.toEqual(weekWindow(47, 600));
  });

  it("always reaches past the six weeks in view", () => {
    for (let top = 0; top < 100; top += 1) {
      const [from, to] = weekWindow(top, 600);
      expect(from).toBeLessThanOrEqual(top);
      expect(to).toBeGreaterThanOrEqual(top + 6);
    }
  });
});
