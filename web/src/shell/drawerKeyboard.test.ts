import { expect, it } from "vitest";
import { revealedScrollTop } from "./drawerKeyboard";

const room = { top: 60, bottom: 540 };

it("leaves a form that's already in the room", () => {
  expect(revealedScrollTop(100, { top: 200, bottom: 400 }, room)).toBe(100);
});

it("scrolls a form under the keyboard up to sit on it", () => {
  expect(revealedScrollTop(100, { top: 500, bottom: 770 }, room)).toBe(330);
});

it("scrolls a form above the room down into it", () => {
  expect(revealedScrollTop(300, { top: 20, bottom: 120 }, room)).toBe(260);
});

it("keeps the top of a form taller than the room in view", () => {
  expect(revealedScrollTop(0, { top: 300, bottom: 900 }, room)).toBe(240);
});
