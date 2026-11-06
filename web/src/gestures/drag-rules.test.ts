import { describe, expect, it } from "vitest";
import { EDGE_ZONE, edgeScroll, isMove, MAX_SCROLL, MOUSE_SLOP, movedBeyond, TOUCH_SLOP } from "./drag-rules";

describe("when a press has become a drag", () => {
  it("takes more than the slop, in any direction", () => {
    expect(movedBeyond(0, 0, TOUCH_SLOP)).toBe(false);
    expect(movedBeyond(7, 7, TOUCH_SLOP)).toBe(false);
    expect(movedBeyond(8, 8, TOUCH_SLOP)).toBe(true);
    expect(movedBeyond(0, -11, TOUCH_SLOP)).toBe(true);
    expect(movedBeyond(5, 4, MOUSE_SLOP)).toBe(true);
    expect(movedBeyond(3, 3, MOUSE_SLOP)).toBe(false);
  });
});

describe("scrolling under a drag", () => {
  const top = 100;
  const bottom = 800;

  it("is still in the middle of the list", () => {
    expect(edgeScroll(450, top, bottom)).toBe(0);
    expect(edgeScroll(top + EDGE_ZONE, top, bottom)).toBe(0);
    expect(edgeScroll(bottom - EDGE_ZONE, top, bottom)).toBe(0);
  });

  it("goes up near the top and down near the bottom, faster the nearer the edge", () => {
    const slow = edgeScroll(top + EDGE_ZONE - 10, top, bottom);
    const fast = edgeScroll(top + 10, top, bottom);
    expect(slow).toBeLessThan(0);
    expect(fast).toBeLessThan(slow);
    const down = edgeScroll(bottom - 10, top, bottom);
    expect(down).toBeGreaterThan(edgeScroll(bottom - EDGE_ZONE + 10, top, bottom));
    expect(down).toBeGreaterThan(0);
  });

  it("tops out at the fastest, also past the edge", () => {
    expect(edgeScroll(top - 200, top, bottom)).toBe(-MAX_SCROLL);
    expect(edgeScroll(bottom + 200, top, bottom)).toBe(MAX_SCROLL);
  });
});

describe("a drop", () => {
  it("is a move only when the place differs", () => {
    expect(isMove({ project: "p", section: "" }, { project: "p", section: "" })).toBe(false);
    expect(isMove({ project: "p", section: "" }, { project: "p", section: "s" })).toBe(true);
    expect(isMove({ project: "p", section: "s" }, { project: "q", section: "s" })).toBe(true);
  });
});
