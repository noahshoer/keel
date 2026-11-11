import { expect, it } from "vitest";
import { keyboardInset } from "./keyboardInset";

it("is 0 with no keyboard", () => {
  expect(keyboardInset(812, 812, 0)).toBe(0);
});

it("is the part of the layout viewport's bottom the keyboard covers", () => {
  expect(keyboardInset(812, 476, 0)).toBe(336);
});

it("counts a visual viewport that has scrolled down", () => {
  expect(keyboardInset(812, 476, 100)).toBe(236);
});

it("rounds, and is never negative", () => {
  expect(keyboardInset(812, 475.6, 0)).toBe(336);
  expect(keyboardInset(812, 820, 0)).toBe(0);
  expect(keyboardInset(812, 700, 200)).toBe(0);
});
