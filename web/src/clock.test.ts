import { expect, test } from "vitest";
import { daySubtitle, localDateOf, localToday } from "./clock";

test("today is the local calendar date, zero-padded", () => {
  expect(localToday(new Date(2026, 0, 5, 9, 0))).toBe("2026-01-05");
  expect(localToday(new Date(2026, 9, 29, 0, 0))).toBe("2026-10-29");
});

test("late evening stays on the local day, not the UTC one", () => {
  // 23:59 local is a different UTC date in every zone west of UTC.
  expect(localToday(new Date(2026, 11, 31, 23, 59))).toBe("2026-12-31");
});

test("the Today subtitle names the weekday and date", () => {
  expect(daySubtitle(new Date(2026, 9, 29, 12, 0))).toBe("Thursday 29 Oct");
  expect(daySubtitle(new Date(2026, 10, 1, 12, 0))).toBe("Sunday 1 Nov");
});

test("a timestamp's local calendar date", () => {
  const at = new Date(2026, 9, 30, 23, 30);
  expect(localDateOf(at.toISOString())).toBe("2026-10-30");
  expect(localDateOf("not a date")).toBeNull();
});
