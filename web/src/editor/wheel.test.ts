import { describe, expect, it } from "vitest";
import { flickTarget, ROW, wrapOffset } from "./wheel";

describe("month wheel", () => {
  it("rows are 36px like .wheel-item", () => {
    expect(ROW).toBe(36);
  });

  it("wraps each row's distance from the band", () => {
    expect(wrapOffset(11, 0, 12)).toBe(-1);
    expect(wrapOffset(1, 11.5, 12)).toBe(1.5);
    expect(wrapOffset(3, 3, 12)).toBe(0);
  });

  it("a flick coasts a third of a second's worth", () => {
    expect(flickTarget(3.2, 0)).toBe(3);
    expect(flickTarget(3, 0.5)).toBe(-1);
  });
});
