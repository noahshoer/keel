// The task card sits on the keyboard, and its pickers on the card. How far up
// the keyboard reaches is sheet-rules' `keyboardInset`, shared with the task
// view's sheet.
import { useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { KEYBOARD_EVENT, keyboardInset } from "../gestures/sheet-rules";
import type { Rect } from "./popover";

const MARGIN = 8;
/** A picker keeps this much room, even above a very tall card. */
const MIN_ROOM = 120;

/**
 * Where a picker sits: above the task card, inside the side margins. It never
 * rises past `minTop`, the top of what's visible; `maxHeight` is its room, in
 * which a taller picker scrolls.
 */
export function placeAboveCard(
  card: Rect,
  pop: Rect,
  vw: number,
  minTop: number = MARGIN,
): { top: number; left: number; maxHeight: number } {
  const bottom = card.top - MARGIN;
  return {
    top: Math.max(minTop, bottom - pop.height),
    left: Math.max(MARGIN, (vw - pop.width) / 2),
    maxHeight: Math.max(bottom - minTop, MIN_ROOM),
  };
}

/** The most of the card's text that stays in view under a picker. */
const MAX_TEXT = 84;
/** Two lines of title: what's left of the text above the tallest picker. */
const MIN_TEXT = 48;

/**
 * How tall the card's text may be while a picker is open: `MAX_TEXT`, less
 * whatever the picker needs to fit between the card and `minTop`. `chrome`
 * is the card's height without its text.
 */
export function textRoomUnderPicker(
  cardBottom: number,
  minTop: number,
  picker: number,
  chrome: number,
): number {
  const left = cardBottom - minTop - MARGIN - picker - chrome;
  return Math.max(MIN_TEXT, Math.min(MAX_TEXT, left));
}

/** `room` is the height of what's visible, which the task card has to fit in. */
export type KeyboardState = { inset: number; up: boolean; room: number };

/** The keyboard as Swift reports it starting to move (`window.keelKeyboard`), as `KEYBOARD_EVENT` carries it. */
export type KeyboardMove = { kb: number; secs: number; ease: string; reduced: boolean };

type Pace = { secs: number; ease: string };

/** The first keyboard after launch comes with no pace (iOS reports 0 s): the pace of every other one, 10% slower than iOS. */
export const FALLBACK_SECS = 0.426;

/** How long a move takes and its curve: the keyboard's own, `FALLBACK_SECS` when iOS gave none, none under reduced motion. */
export function paceOf(m: KeyboardMove): Pace {
  if (m.reduced) return { secs: 0, ease: m.ease };
  return { secs: m.secs > 0 ? m.secs : FALLBACK_SECS, ease: m.ease };
}

/**
 * The card's move at `pace`, on its transform: the compositor's, so it keeps up
 * with the keyboard (`bottom` is layout, on the page's thread). `null` when it
 * doesn't move.
 */
export function riseTransition(pace: Pace): string | null {
  if (pace.secs <= 0) return null;
  return `transform ${pace.secs}s ${pace.ease}`;
}

/**
 * Starts `card` from where it was shown (`from`, its bottom edge before the
 * layout changed) and has it ride its transform to where the layout now puts
 * it, at `pace`. Returns the timer that puts its style back afterwards.
 */
export function rideCard(
  card: HTMLElement,
  from: number,
  pace: Pace,
): ReturnType<typeof setTimeout> | undefined {
  const transition = riseTransition(pace);
  const delta = from - card.getBoundingClientRect().bottom;
  if (transition === null || delta === 0) return undefined;
  card.style.transition = "none";
  card.style.transform = `translateY(${delta}px)`;
  // The place it starts from has to be laid out before the transition is set.
  card.getBoundingClientRect();
  card.style.transition = transition;
  card.style.transform = "";
  return setTimeout(
    () => {
      card.style.transition = "";
    },
    pace.secs * 1000 + 50,
  );
}

/** Whether the visible area is shorter than the page: the keyboard is up. */
export function keyboardUp(v: VisualViewport): boolean {
  return v.height < document.documentElement.clientHeight - 0.5;
}

/**
 * The keyboard from the layout height and the visual viewport. While one is
 * still coming up (`held`, its height) the viewport hasn't resized: the card
 * already counts as on the keyboard, with the room left above it.
 */
export function readKeyboard(
  clientH: number,
  vvTop: number,
  vvH: number,
  held: number | null,
): KeyboardState {
  const inset = keyboardInset(clientH, vvTop, vvH, held);
  const resized = vvH < clientH - 0.5;
  if (held !== null && !resized) return { inset, up: true, room: vvH - inset };
  return { inset, up: resized, room: vvH };
}

/**
 * The keyboard's reach while `enabled`, updated as the visual viewport moves.
 * `paced` (the task card) also follows the keyboard's own start, which comes
 * about 420 ms before the viewport resizes: the inset is its height at once,
 * and the card rides to it at the keyboard's pace (`rideCard`).
 */
export function useKeyboard(enabled: boolean, paced = false): KeyboardState {
  const held = useRef<{ kb: number; until: number } | null>(null);
  const read = (): KeyboardState => {
    const v = window.visualViewport;
    if (!v) return { inset: 0, up: false, room: innerHeight };
    const kb = held.current !== null && Date.now() < held.current.until ? held.current.kb : null;
    return readKeyboard(document.documentElement.clientHeight, v.offsetTop, v.height, kb);
  };
  const [state, setState] = useState<KeyboardState>(read);
  // biome-ignore lint/correctness/useExhaustiveDependencies: `read` only reads a ref and the window.
  useEffect(() => {
    const v = window.visualViewport;
    if (!enabled || !v) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const update = () => setState(read());
    const onMove = (e: Event) => {
      const m = (e as CustomEvent<KeyboardMove>).detail;
      const pace = paceOf(m);
      held.current = m.kb > 0 ? { kb: m.kb, until: Date.now() + pace.secs * 1000 + 500 } : null;
      const card = document.querySelector<HTMLElement>(".editor.on-keyboard");
      const from = card?.getBoundingClientRect().bottom;
      // Now, not with the next render: the card has to start with the keyboard.
      flushSync(update);
      if (card && from !== undefined) {
        clearTimeout(timer);
        timer = rideCard(card, from, pace);
      }
    };
    update();
    v.addEventListener("resize", update);
    v.addEventListener("scroll", update);
    if (paced) window.addEventListener(KEYBOARD_EVENT, onMove);
    return () => {
      clearTimeout(timer);
      held.current = null;
      v.removeEventListener("resize", update);
      v.removeEventListener("scroll", update);
      window.removeEventListener(KEYBOARD_EVENT, onMove);
    };
  }, [enabled, paced]);
  return state;
}

/**
 * The inset to stay at instead of the keyboard's. It's the height when `holding`
 * began, kept until the keyboard is back up to it or gone: after `holding` ends
 * the taller keyboard is still on its way.
 */
export function nextHeld(held: number | null, inset: number, holding: boolean, up: boolean): number | null {
  if (holding) return Math.max(held ?? 0, inset);
  return held === null || !up || inset >= held ? null : held;
}

/**
 * The keyboard's reach, staying put while `holding`: the number pad is shorter
 * than the letters, and the card would drop to it and rise again.
 */
export function useHeldKeyboard(keyboard: KeyboardState, holding: boolean): KeyboardState {
  const held = useRef<number | null>(null);
  held.current = nextHeld(held.current, keyboard.inset, holding, keyboard.up);
  if (held.current === null) return keyboard;
  return { ...keyboard, inset: held.current, room: keyboard.room - (held.current - keyboard.inset) };
}
