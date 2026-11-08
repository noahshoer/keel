import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { watchDayChange } from "./dayChange";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

test("fires once the local date rolls over at midnight", () => {
  vi.setSystemTime(new Date(2026, 10, 8, 23, 59, 30));
  const onChange = vi.fn();
  const stop = watchDayChange(onChange);
  vi.advanceTimersByTime(29_000);
  expect(onChange).not.toHaveBeenCalled();
  vi.advanceTimersByTime(2_000);
  expect(onChange).toHaveBeenCalledTimes(1);
  // And again the next midnight.
  vi.advanceTimersByTime(24 * 3600_000);
  expect(onChange).toHaveBeenCalledTimes(2);
  stop();
});

test("a wake-up after the day changed (timers frozen) fires on return to the page", () => {
  vi.setSystemTime(new Date(2026, 10, 8, 22, 0, 0));
  const onChange = vi.fn();
  const stop = watchDayChange(onChange);
  vi.setSystemTime(new Date(2026, 10, 9, 8, 0, 0)); // timer never ran
  document.dispatchEvent(new Event("visibilitychange"));
  expect(onChange).toHaveBeenCalledTimes(1);
  document.dispatchEvent(new Event("visibilitychange"));
  expect(onChange).toHaveBeenCalledTimes(1);
  stop();
});

test("stopping cancels the timer", () => {
  vi.setSystemTime(new Date(2026, 10, 8, 23, 59, 59));
  const onChange = vi.fn();
  watchDayChange(onChange)();
  vi.advanceTimersByTime(5_000);
  expect(onChange).not.toHaveBeenCalled();
});
