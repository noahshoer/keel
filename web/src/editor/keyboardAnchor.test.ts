import { describe, expect, it } from "vitest";
import { keyboardInset } from "../gestures/sheet-rules";
import { placeAboveCard } from "./keyboardAnchor";
import type { Rect } from "./popover";

const rect = (left: number, top: number, width: number, height: number): Rect => ({
  left,
  top,
  width,
  height,
  bottom: top + height,
});

describe("the task card's inset", () => {
  it("is the keyboard's height", () => {
    expect(keyboardInset(844, 0, 544, null)).toBe(300);
  });

  it("is zero once the keyboard closes", () => {
    expect(keyboardInset(844, 0, 844, null)).toBe(0);
  });

  it("allows for a scrolled viewport, and is zero when WebKit scrolled the whole way", () => {
    expect(keyboardInset(844, 20, 544, null)).toBe(280);
    expect(keyboardInset(874, 396, 478, null)).toBe(0);
  });
});

describe("placeAboveCard", () => {
  const card = rect(0, 500, 402, 136);

  it("sits 8px above the card, inside the side margins", () => {
    expect(placeAboveCard(card, rect(0, 0, 386, 200), 402)).toEqual({ top: 292, left: 8, maxHeight: 484 });
  });

  it("clamps to the top of the screen when the picker is tall", () => {
    expect(placeAboveCard(card, rect(0, 0, 386, 600), 402)).toEqual({ top: 8, left: 8, maxHeight: 484 });
  });

  it("always leaves a picker room, even above a very tall card", () => {
    const tall = rect(0, 100, 402, 500);
    const at = placeAboveCard(tall, rect(0, 0, 386, 300), 402, 400);
    expect(at.maxHeight).toBe(120);
  });

  it("keeps a tall picker below the visible top, leaving it the room to scroll in", () => {
    expect(placeAboveCard(card, rect(0, 0, 386, 383), 402, 200)).toEqual({
      top: 200,
      left: 8,
      maxHeight: 292,
    });
  });
});
