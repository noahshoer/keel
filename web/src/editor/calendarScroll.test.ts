import { describe, expect, it } from "vitest";
import { monthInView, weekWindow } from "./calendarScroll";

describe("the calendar's scroll", () => {
  // Rows of 40, six in view; the first month has five weeks, the second four.
  const row = 40;
  const tops = [0, 200, 360];

  it("settled on a month's 1st, that month is the shown one, even a four-week month", () => {
    expect(monthInView(tops, 0, row)).toBe(0);
    expect(monthInView(tops, 200, row)).toBe(1);
    expect(monthInView(tops, 360, row)).toBe(2);
  });

  it("the next month takes over once its 1st reaches the middle of the view", () => {
    // The 1st of the second month is at row 3 when the view's top is at 80.
    expect(monthInView(tops, 79, row)).toBe(0);
    expect(monthInView(tops, 80, row)).toBe(1);
  });

  it("a fraction of a pixel short still counts", () => {
    expect(monthInView(tops, 79.6, row)).toBe(1);
  });

  it("stays on the last month past the end", () => {
    expect(monthInView(tops, 5000, row)).toBe(2);
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
