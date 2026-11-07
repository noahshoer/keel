import { describe, expect, it } from "vitest";
import { gluedPath, springPoints, translateY } from "./glide";

describe("the keyboard's curve as points", () => {
  it("reads a sampled spring", () => {
    expect(springPoints("linear(0,0.0230,0.5,1)")).toEqual([0, 0.023, 0.5, 1]);
    expect(springPoints("linear(0, 0.4, 1)")).toEqual([0, 0.4, 1]);
  });

  it("is nothing for any other curve", () => {
    expect(springPoints("cubic-bezier(.25,.9,.3,1)")).toBeNull();
    expect(springPoints("ease")).toBeNull();
    expect(springPoints("linear(0)")).toBeNull();
  });
});

// A 874 pt screen: the keyboard's top goes from 874 to 546. The card rests
// with its bottom at 832 (above the home indicator) and sits 8 pt above the
// keyboard once it's up, at 538.
const show = { kbFrom: 874, kbTo: 546, gap: 8, rest: 832, from: 832, final: 538 };

describe("the card's path on the keyboard", () => {
  it("waits where it rests until the keyboard reaches it, then rides on it", () => {
    // Keyboard tops 874, 841.2, 710, 546: the card is picked up between the
    // first two and is 8 pt above the keyboard from then on.
    expect(gluedPath({ ...show, points: [0, 0.1, 0.5, 1] })).toEqual([294, 294, 164, 0]);
  });

  it("rides down on the keyboard and stops where it rests", () => {
    const hide = { kbFrom: 546, kbTo: 874, gap: 8, rest: 832, from: 538, final: 832 };
    expect(gluedPath({ ...hide, points: [0, 0.5, 0.9, 1] })).toEqual([-294, -130, 0, 0]);
  });

  it("starts from where the card is when that's off the keyboard, and still ends in place", () => {
    // Mid-way through another move: 100 pt higher than the path's start.
    const path = gluedPath({ ...show, from: 732, points: [0, 0.5, 1] });
    expect(path[0]).toBe(194);
    expect(path[2]).toBe(0);
    // The difference fades as the keyboard goes: half of it at half way.
    expect(path[1]).toBe(164 - 50);
  });
});

describe("a transform's vertical move", () => {
  it("reads a matrix's last value", () => {
    expect(translateY("matrix(1, 0, 0, 1, 0, -42.5)")).toBe(-42.5);
  });

  it("is zero without a transform", () => {
    expect(translateY("none")).toBe(0);
    expect(translateY("")).toBe(0);
  });
});
