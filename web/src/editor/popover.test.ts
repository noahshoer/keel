import { describe, expect, it } from "vitest";
import { placePopover, type Rect } from "./popover";

const rect = (left: number, top: number, width: number, height: number): Rect => ({
  left,
  top,
  width,
  height,
  bottom: top + height,
});

describe("placePopover", () => {
  const pop = rect(0, 0, 280, 300);

  it("sits 6px below its chip", () => {
    expect(placePopover(rect(20, 100, 80, 30), pop, 1200, 900)).toEqual({ top: 136, left: 20 });
  });

  it("flips above when there's no room below", () => {
    expect(placePopover(rect(20, 700, 80, 30), pop, 1200, 900)).toEqual({ top: 394, left: 20 });
  });

  it("shifts left at the right edge of a phone", () => {
    expect(placePopover(rect(300, 100, 60, 30), pop, 375, 812).left).toBe(87);
  });

  it("clamps to the margin when neither side fits", () => {
    expect(placePopover(rect(20, 200, 80, 30), rect(0, 0, 280, 600), 375, 500).top).toBe(8);
  });
});
