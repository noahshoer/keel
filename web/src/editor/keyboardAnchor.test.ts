import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { KEYBOARD_EVENT, keyboardInset } from "../gestures/sheet-rules";
import {
  FALLBACK_SECS,
  nextHeld,
  paceOf,
  placeAboveCard,
  readKeyboard,
  rideCard,
  riseTransition,
  textRoomUnderPicker,
  useKeyboard,
} from "./keyboardAnchor";
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

describe("textRoomUnderPicker", () => {
  // A card ending at 538 with 84 of pills and padding, under a status bar ending at 70.
  const room = (picker: number) => textRoomUnderPicker(538, 70, picker, 84);

  it("is its most when the picker fits above that", () => {
    expect(room(206)).toBe(84);
  });

  it("shrinks for a taller picker, so it fits without scrolling", () => {
    expect(room(298)).toBe(78);
    expect(538 - room(298) - 84 - 8 - 298).toBe(70);
  });

  it("keeps two lines of text, whatever the picker's height", () => {
    expect(room(900)).toBe(48);
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

describe("nextHeld", () => {
  const LETTERS = 336;
  const PAD = 310;

  it("keeps the height the letters reached while the number pad is up", () => {
    const held = nextHeld(null, LETTERS, true, true);
    expect(held).toBe(LETTERS);
    expect(nextHeld(held, PAD, true, true)).toBe(LETTERS);
  });

  it("lets go only once the letters are back, so closing doesn't dip", () => {
    expect(nextHeld(LETTERS, PAD, false, true)).toBe(LETTERS);
    expect(nextHeld(LETTERS, LETTERS, false, true)).toBeNull();
  });

  it("lets go if the keyboard goes away", () => {
    expect(nextHeld(LETTERS, 0, false, false)).toBeNull();
  });

  it("holds nothing when nothing asks it to", () => {
    expect(nextHeld(null, LETTERS, false, true)).toBeNull();
  });
});

describe("the keyboard as the card reads it", () => {
  it("is the viewport's, with no keyboard still coming up", () => {
    expect(readKeyboard(844, 0, 844, null)).toEqual({ inset: 0, up: false, room: 844 });
    expect(readKeyboard(844, 0, 544, null)).toEqual({ inset: 300, up: true, room: 544 });
  });

  it("is the held height while the viewport hasn't resized: up, with the room left above it", () => {
    expect(readKeyboard(844, 0, 844, 328)).toEqual({ inset: 328, up: true, room: 516 });
  });

  it("goes back to the viewport once it has resized", () => {
    expect(readKeyboard(844, 0, 516, 328)).toEqual({ inset: 328, up: true, room: 516 });
    expect(readKeyboard(844, 20, 500, 328)).toEqual({ inset: 324, up: true, room: 500 });
  });
});

describe("the pace of a keyboard move", () => {
  it("is the keyboard's own, a default when iOS reports none, and none under reduced motion", () => {
    expect(paceOf({ kb: 328, secs: 0.38, ease: "ease", reduced: false })).toEqual({
      secs: 0.38,
      ease: "ease",
    });
    expect(paceOf({ kb: 328, secs: 0, ease: "ease", reduced: false })).toEqual({
      secs: FALLBACK_SECS,
      ease: "ease",
    });
    expect(paceOf({ kb: 328, secs: 0.38, ease: "ease", reduced: true })).toEqual({ secs: 0, ease: "ease" });
  });

  it("moves the card by transform (the compositor's, as the sheet's), or not at all", () => {
    expect(riseTransition({ secs: 0.4, ease: "ease" })).toBe("transform 0.4s ease");
    expect(riseTransition({ secs: 0, ease: "ease" })).toBeNull();
  });
});

describe("useKeyboard with the pace", () => {
  const send = (detail: object) =>
    act(() => window.dispatchEvent(new CustomEvent(KEYBOARD_EVENT, { detail })));

  beforeEach(() => {
    vi.useFakeTimers();
    Object.defineProperty(document.documentElement, "clientHeight", { value: 844, configurable: true });
    const vv = Object.assign(new EventTarget(), { offsetTop: 0, height: 844, scale: 1 });
    vi.stubGlobal("visualViewport", vv);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("sits on the keyboard at once, and lets the viewport take over", () => {
    const { result } = renderHook(() => useKeyboard(true, true));
    expect(result.current).toMatchObject({ inset: 0, up: false });
    send({ kb: 328, secs: 0.4, ease: "ease", reduced: false });
    expect(result.current).toMatchObject({ inset: 328, up: true });
    act(() => {
      vi.advanceTimersByTime(451);
    });
    expect(result.current.inset).toBe(328);
  });

  it("goes down with the keyboard", () => {
    const { result } = renderHook(() => useKeyboard(true, true));
    send({ kb: 328, secs: 0.4, ease: "ease", reduced: false });
    (window.visualViewport as unknown as { height: number }).height = 516;
    act(() => window.visualViewport?.dispatchEvent(new Event("resize")));
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(result.current.inset).toBe(328);
    send({ kb: 0, secs: 0.4, ease: "ease", reduced: false });
    (window.visualViewport as unknown as { height: number }).height = 844;
    act(() => window.visualViewport?.dispatchEvent(new Event("resize")));
    expect(result.current).toMatchObject({ inset: 0, up: false });
  });

  it("starts the card from where it was shown, riding on its transform", () => {
    document.body.insertAdjacentHTML("beforeend", '<form class="editor on-keyboard" id="card"></form>');
    const card = document.getElementById("card") as HTMLElement;
    // Low on the screen before the move, 328 up after it.
    let bottom = 836;
    card.getBoundingClientRect = () => ({ bottom }) as DOMRect;
    const { result } = renderHook(() => useKeyboard(true, true));
    bottom = 836;
    const stages: string[] = [];
    const real = card.style;
    Object.defineProperty(card, "style", {
      value: new Proxy(real, {
        set(target, key, value) {
          if (key === "transform" || key === "transition") stages.push(`${String(key)}=${value}`);
          return Reflect.set(target, key, value);
        },
      }),
    });
    // The layout the keyboard's move leads to.
    const stub = vi.spyOn(card, "getBoundingClientRect");
    stub.mockImplementationOnce(() => ({ bottom: 836 }) as DOMRect);
    stub.mockImplementation(() => ({ bottom: 508 }) as DOMRect);
    send({ kb: 328, secs: 0.4, ease: "ease", reduced: false });
    expect(result.current.inset).toBe(328);
    expect(stages).toEqual([
      "transition=none",
      "transform=translateY(328px)",
      "transition=transform 0.4s ease",
      "transform=",
    ]);
    // Back to plain once the move is over.
    act(() => {
      vi.advanceTimersByTime(451);
    });
    expect(stages.at(-1)).toBe("transition=");
    card.remove();
  });

  it("ignores the move when not paced (the drawer's)", () => {
    const { result } = renderHook(() => useKeyboard(true));
    send({ kb: 328, secs: 0.4, ease: "ease", reduced: false });
    expect(result.current).toMatchObject({ inset: 0, up: false });
  });
});
