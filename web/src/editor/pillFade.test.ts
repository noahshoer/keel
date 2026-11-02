import { expect, it } from "vitest";
import { pillsRunBack, pillsRunOn } from "./pillFade";

it("runs back once the row is scrolled from its start", () => {
  expect(pillsRunBack(0)).toBe(false);
  expect(pillsRunBack(40)).toBe(true);
});

it("is true while a pill runs past the end of the row", () => {
  expect(pillsRunOn(0, 300, 700)).toBe(true);
  expect(pillsRunOn(100, 300, 700)).toBe(true);
});

it("is false once the last pill has cleared it, or when they all fit", () => {
  expect(pillsRunOn(400, 300, 700)).toBe(false);
  expect(pillsRunOn(0, 300, 300)).toBe(false);
});
