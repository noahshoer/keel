import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { makeTimers } from "./timers";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

it("runs a callback once after its delay", () => {
  const t = makeTimers();
  const f = vi.fn();
  t.later(f, 100);
  vi.advanceTimersByTime(99);
  expect(f).not.toHaveBeenCalled();
  vi.advanceTimersByTime(1);
  expect(f).toHaveBeenCalledTimes(1);
  vi.advanceTimersByTime(1000);
  expect(f).toHaveBeenCalledTimes(1);
});

it("clear cancels everything waiting, and only that", () => {
  const t = makeTimers();
  const a = vi.fn();
  const b = vi.fn();
  t.later(a, 10);
  t.later(b, 20);
  t.clear();
  vi.advanceTimersByTime(100);
  expect(a).not.toHaveBeenCalled();
  expect(b).not.toHaveBeenCalled();
  t.later(a, 10);
  vi.advanceTimersByTime(10);
  expect(a).toHaveBeenCalledTimes(1);
});

it("a callback may schedule another", () => {
  const t = makeTimers();
  const log: string[] = [];
  t.later(() => {
    log.push("a");
    t.later(() => log.push("b"), 10);
  }, 10);
  vi.advanceTimersByTime(20);
  expect(log).toEqual(["a", "b"]);
});
