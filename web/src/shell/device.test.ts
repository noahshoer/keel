import { afterEach, expect, it, vi } from "vitest";
import { isTouch, NARROW_QUERY, prefersReducedMotion, WIDE_MIN, WIDE_QUERY } from "./device";

afterEach(() => vi.unstubAllGlobals());

const win = (matches: boolean) => {
  const matchMedia = vi.fn(() => ({ matches }));
  return { matchMedia } as unknown as Window;
};

it("reads the reduced-motion preference from the given window", () => {
  const on = win(true);
  expect(prefersReducedMotion(on)).toBe(true);
  expect(on.matchMedia).toHaveBeenCalledWith("(prefers-reduced-motion: reduce)");
  expect(prefersReducedMotion(win(false))).toBe(false);
});

it("is false without a window or without matchMedia", () => {
  expect(prefersReducedMotion(null)).toBe(false);
  expect(prefersReducedMotion(undefined as unknown as Window)).toBe(false);
  expect(prefersReducedMotion({} as Window)).toBe(false);
});

it("defaults to the global window", () => {
  vi.stubGlobal("matchMedia", () => ({ matches: true }));
  expect(prefersReducedMotion()).toBe(true);
});

it("names the width breakpoint once", () => {
  expect(WIDE_MIN).toBe(900);
  expect(WIDE_QUERY).toBe("(min-width: 900px)");
  expect(NARROW_QUERY).toBe("(max-width: 899px)");
});

it("isTouch follows a coarse pointer, and is false without matchMedia", () => {
  vi.stubGlobal("matchMedia", (q: string) => ({ matches: q === "(pointer: coarse)" }));
  expect(isTouch()).toBe(true);
  vi.stubGlobal("matchMedia", undefined);
  expect(isTouch()).toBe(false);
});
