import { expect, it } from "vitest";
import { placeRowMenu } from "./placeRowMenu";

const row = (top: number) => ({ top, bottom: top + 44, left: 12, width: 330, height: 44 });
const menu = { top: 0, bottom: 0, left: 0, width: 210, height: 100 };

it("opens under a row with room below, against its right edge", () => {
  expect(placeRowMenu(row(200), menu, 402, 874)).toEqual({ top: 250, left: 132 });
});

it("opens above a row too near the bottom", () => {
  expect(placeRowMenu(row(780), menu, 402, 874)).toEqual({ top: 674, left: 132 });
});

it("stays on screen when it's wider than the row", () => {
  expect(placeRowMenu({ ...row(200), width: 100 }, menu, 402, 874).left).toBe(8);
});
