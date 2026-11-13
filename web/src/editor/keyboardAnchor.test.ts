import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { KEYBOARD_EVENT, keyboardInset } from "../gestures/sheet-rules";
import { rect } from "../test/dom";
import {
  afterKeyboardMove,
  KEYBOARD_MOVE_LIMIT_MS,
  type KeyboardMove,
  lowerKeyboardNow,
  nextHeld,
  paceOf,
  placeAboveCard,
  readKeyboard,
  rideCard,
  SNAPPED_RISE_SECS,
  textRoomUnderPicker,
  useKeyboard,
} from "./keyboardAnchor";

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

  it("counts a keyboard on its way down as down, before the viewport resizes", () => {
    expect(readKeyboard(844, 0, 516, 0)).toEqual({ inset: 0, up: false, room: 844 });
  });
});

describe("a picker open on the card as it rides", () => {
  it("holds its place on the screen while the card moves under it", () => {
    document.body.insertAdjacentHTML(
      "beforeend",
      '<form class="editor on-keyboard" id="ride-card"><div class="popover" id="ride-pop"></div></form>',
    );
    const card = document.getElementById("ride-card") as HTMLElement;
    const pop = document.getElementById("ride-pop") as HTMLElement;
    const moves = new Map<HTMLElement, string[]>();
    for (const el of [card, pop]) {
      Object.assign(el, {
        animate: (k: Keyframe[]) => {
          moves.set(
            el,
            k.map((f) => String(f["transform"])),
          );
          return {} as Animation;
        },
        getAnimations: () => [],
      });
    }
    // The calendar lowers the keyboard: the card rides down from 538 to rest at 832.
    card.getBoundingClientRect = () => ({ bottom: 832 }) as DOMRect;
    rideCard(
      card,
      538,
      { secs: 0.4, ease: "linear(0,0.5,1)" },
      { kbFrom: 546, kbTo: 874, gap: 8, rest: 832 },
    );
    expect(moves.get(card)).toEqual(["translateY(-294px)", "translateY(-130px)", "translateY(0px)"]);
    // The picker moves the other way, so it stays where it was placed.
    expect(moves.get(pop)).toEqual(["translateY(294px)", "translateY(130px)", "translateY(0px)"]);
    card.remove();
  });
});

describe("the pace of a keyboard move", () => {
  it("is the keyboard's own, and none when it doesn't animate or under reduced motion", () => {
    expect(paceOf({ kb: 328, secs: 0.38, ease: "ease", reduced: false })).toEqual({
      secs: 0.38,
      ease: "ease",
    });
    // A very quick first tap after launch snaps the keyboard in (iOS reports 0 s):
    // the card still rises into place, in 0.15 s, so it arrives with it. Only then.
    expect(SNAPPED_RISE_SECS).toBe(0.15);
    expect(paceOf({ kb: 328, secs: 0, ease: "ease", reduced: false })).toEqual({
      secs: SNAPPED_RISE_SECS,
      ease: "ease",
    });
    // One that snaps away: the card goes with it.
    expect(paceOf({ kb: 0, secs: 0, ease: "ease", reduced: false })).toEqual({ secs: 0, ease: "ease" });
    expect(paceOf({ kb: 328, secs: 0.38, ease: "ease", reduced: true })).toEqual({ secs: 0, ease: "ease" });
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

  it("rides the keyboard's own path on the keyboard's spring, from where it rests", () => {
    document.body.insertAdjacentHTML("beforeend", '<form class="editor on-keyboard" id="card3"></form>');
    const card = document.getElementById("card3") as HTMLElement;
    const frames: Keyframe[][] = [];
    const options: KeyframeAnimationOptions[] = [];
    Object.assign(card, {
      animate: (k: Keyframe[], o: KeyframeAnimationOptions) => {
        frames.push(k);
        options.push(o);
        return {} as Animation;
      },
      getAnimations: () => [],
    });
    // At rest above a 34 pt home indicator (844 - 34 - 8), then 8 pt above the keyboard.
    const stub = vi.spyOn(card, "getBoundingClientRect");
    stub.mockImplementationOnce(() => ({ bottom: 802 }) as DOMRect);
    stub.mockImplementation(() => ({ bottom: 508 }) as DOMRect);
    const margin = vi.spyOn(window, "getComputedStyle");
    margin.mockImplementationOnce(() => ({ marginBottom: "34px" }) as CSSStyleDeclaration);
    margin.mockImplementation(() => ({ marginBottom: "0px" }) as CSSStyleDeclaration);
    renderHook(() => useKeyboard(true, true));
    send({ kb: 328, secs: 0.4, ease: "linear(0,0.1,0.5,1)", reduced: false });
    margin.mockRestore();
    // Keyboard tops 844, 811.2, 680, 516: picked up after the first tenth.
    expect(frames[0]?.map((f) => f["transform"])).toEqual([
      "translateY(294px)",
      "translateY(294px)",
      "translateY(164px)",
      "translateY(0px)",
    ]);
    expect(options[0]).toEqual({ id: "keel-ride", duration: 400, easing: "linear" });
    expect(card.classList.contains("riding")).toBe(true);
    // iOS announces a keyboard more than once: the ride under way goes on.
    send({ kb: 328, secs: 0.4, ease: "linear(0,0.1,0.5,1)", reduced: false });
    expect(frames).toHaveLength(1);
    act(() => {
      vi.advanceTimersByTime(451);
    });
    expect(card.classList.contains("riding")).toBe(false);
    card.remove();
  });

  it("starts a move the page sets off itself at once, at the keyboard's last pace", () => {
    const seen: KeyboardMove[] = [];
    const listen = (e: Event) => seen.push((e as CustomEvent<KeyboardMove>).detail);
    window.addEventListener(KEYBOARD_EVENT, listen);
    renderHook(() => useKeyboard(true, true));
    send({ kb: 328, secs: 0.4, ease: "linear(0,1)", reduced: false });
    lowerKeyboardNow();
    expect(seen.at(-1)).toMatchObject({ kb: 0, secs: 0.4, ease: "linear(0,1)", reduced: false });
    window.removeEventListener(KEYBOARD_EVENT, listen);
  });

  it("ignores the move when not paced (the drawer's)", () => {
    const { result } = renderHook(() => useKeyboard(true));
    send({ kb: 328, secs: 0.4, ease: "ease", reduced: false });
    expect(result.current).toMatchObject({ inset: 0, up: false });
  });
});

describe("work that waits for the keyboard's move", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("runs once the move has started and a frame has gone by", () => {
    const then = vi.fn();
    afterKeyboardMove(then);
    vi.advanceTimersByTime(10);
    expect(then).not.toHaveBeenCalled();
    window.dispatchEvent(new CustomEvent(KEYBOARD_EVENT, { detail: { kb: 0 } }));
    expect(then).not.toHaveBeenCalled();
    vi.advanceTimersByTime(40);
    expect(then).toHaveBeenCalledTimes(1);
  });

  it("runs anyway when no move comes, and only once", () => {
    const then = vi.fn();
    afterKeyboardMove(then);
    vi.advanceTimersByTime(KEYBOARD_MOVE_LIMIT_MS + 40);
    expect(then).toHaveBeenCalledTimes(1);
    window.dispatchEvent(new CustomEvent(KEYBOARD_EVENT, { detail: { kb: 0 } }));
    vi.advanceTimersByTime(100);
    expect(then).toHaveBeenCalledTimes(1);
  });
});
