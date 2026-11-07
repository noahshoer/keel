// The task card sits on the keyboard, and its pickers on the card. How far up
// the keyboard reaches is sheet-rules' `keyboardInset`, shared with the task
// view's sheet.
import { useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { log } from "../diag";
import { KEYBOARD_EVENT, keyboardInset } from "../gestures/sheet-rules";
import { type Glide, gluedPath, springPoints } from "./glide";
import type { Rect } from "./popover";

const MARGIN = 8;
/** How far above the keyboard the task card sits. */
export const CARD_GAP = 8;
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
export type KeyboardMove = {
  kb: number;
  secs: number;
  ease: string;
  reduced: boolean;
  /** When Swift saw the keyboard start (epoch ms), to count the hop to the page. */
  at?: number;
};

type Pace = { secs: number; ease: string };

/**
 * How long the card takes to rise onto a keyboard that snapped in (a very
 * quick tap on + right after launch). Short, so it reads as arriving with the
 * keyboard: at the keyboard's usual 0.43 s it lagged behind one already there.
 */
export const SNAPPED_RISE_SECS = 0.15;

/**
 * How long a move takes and its curve: the keyboard's own. When the keyboard
 * snaps up (iOS reports 0 s, as for the first keyboard after launch after a
 * very quick tap), the card still rises into place, briefly
 * (`SNAPPED_RISE_SECS`), rather than popping in; when one snaps away, the
 * card goes with it. None under reduced motion. It starts with the keyboard,
 * never ahead of it: a head start on the keyboard's stiff spring was a 96 pt
 * jump on the first frame.
 */
export function paceOf(m: KeyboardMove): Pace {
  if (m.reduced) return { secs: 0, ease: m.ease };
  if (m.secs <= 0) return { secs: m.kb > 0 ? SNAPPED_RISE_SECS : 0, ease: m.ease };
  return { secs: m.secs, ease: m.ease };
}

/** The id of a ride's animations, so a new ride cancels only those. */
const RIDE = "keel-ride";

/** The keyboard's move as the card rides it (see `gluedPath`). */
export type Track = Pick<Glide, "kbFrom" | "kbTo" | "gap" | "rest">;

/**
 * Has `card` ride the keyboard from where it was shown (`from`, its bottom
 * edge before the layout changed) to where the layout now puts it. Its
 * transform follows the keyboard's own path (`gluedPath`), sampled from the
 * keyboard's spring (`linear()` from Swift), as keyframes: the compositor's,
 * so it keeps up with the keyboard (`bottom` is layout, on the page's thread).
 * Returns the timer that gives the caret back afterwards.
 */
export function rideCard(
  card: HTMLElement,
  from: number,
  pace: Pace,
  track: Track,
): ReturnType<typeof setTimeout> | undefined {
  const points = springPoints(pace.ease);
  if (pace.secs <= 0 || points === null || typeof card.animate !== "function") return undefined;
  const pickers = [...card.querySelectorAll<HTMLElement>(".popover")];
  // Where the layout puts it, without what's left of an earlier move.
  for (const el of [card, ...pickers]) {
    for (const a of el.getAnimations()) if (a.id === RIDE) a.cancel();
  }
  const path = gluedPath({ ...track, points, from, final: card.getBoundingClientRect().bottom });
  if (path.every((y) => Math.abs(y) < 0.5)) return undefined;
  const timing = { id: RIDE, duration: pace.secs * 1000, easing: "linear" };
  card.animate(
    path.map((y) => ({ transform: `translateY(${y}px)` })),
    timing,
  );
  // A picker open on the card (the calendar, which lowered the keyboard) stays
  // where it was placed while the card moves under it: it's inside the card,
  // so it moves the other way.
  for (const pop of pickers) {
    pop.animate(
      path.map((y) => ({ transform: `translateY(${-y}px)` })),
      timing,
    );
  }
  // iOS draws the caret where the layout is, not where the card is yet.
  card.classList.add("riding");
  return setTimeout(() => card.classList.remove("riding"), pace.secs * 1000 + 50);
}

/** The last move Swift reported with a pace: what a move the page sets off itself rides at. */
let lastPaced: KeyboardMove | null = null;

/**
 * Starts the card down with the keyboard the page is about to lower (by
 * blurring its field), at the keyboard's last pace, instead of waiting for
 * Swift's report, which comes ~30 ms after the keyboard has set off. That
 * report is then the same move, and the ride under way carries on. Whether
 * it started one (there's no pace before the first report).
 */
export function lowerKeyboardNow(): boolean {
  if (lastPaced === null) return false;
  const { secs, ease, reduced } = lastPaced;
  window.dispatchEvent(new CustomEvent(KEYBOARD_EVENT, { detail: { kb: 0, secs, ease, reduced } }));
  return true;
}

/** The most `afterKeyboardMove` waits for a move (none comes when the keyboard wasn't up). */
export const KEYBOARD_MOVE_LIMIT_MS = 200;

/**
 * Runs `then` once the keyboard's next move has started (`KEYBOARD_EVENT`, after
 * the card's ride is set off) and a frame has gone by, so the ride is on the
 * compositor first; or after `KEYBOARD_MOVE_LIMIT_MS` when no move comes. For
 * heavy work set off with the move (the date's calendar, which lowers the
 * keyboard): on the page's thread before the ride, it held the card back
 * ~60 ms while the keyboard went.
 */
export function afterKeyboardMove(then: () => void): void {
  let done = false;
  const run = () => {
    if (done) return;
    done = true;
    window.removeEventListener(KEYBOARD_EVENT, run);
    clearTimeout(limit);
    requestAnimationFrame(() => setTimeout(then, 0));
  };
  window.addEventListener(KEYBOARD_EVENT, run);
  const limit = setTimeout(run, KEYBOARD_MOVE_LIMIT_MS);
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
  // One on its way down (`held` 0): down already, though the viewport is still short.
  if (held === 0) return { inset: 0, up: false, room: clientH };
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
    /** The move under way: iOS announces a keyboard more than once. */
    let moving: { kb: number; until: number } | null = null;
    const onMove = (e: Event) => {
      const m = (e as CustomEvent<KeyboardMove>).detail;
      if (m.secs > 0) lastPaced = m;
      const pace = paceOf(m);
      const now = Date.now();
      log(`card follows kb ${m.kb} secs ${pace.secs} hop ${m.at === undefined ? "?" : now - m.at}ms`);
      if (moving !== null && moving.kb === m.kb && now < moving.until) return;
      moving = { kb: m.kb, until: now + pace.secs * 1000 };
      const clientH = document.documentElement.clientHeight;
      const before = read();
      // Until the viewport has caught up, in either direction.
      held.current = { kb: m.kb, until: now + pace.secs * 1000 + 500 };
      const card = document.querySelector<HTMLElement>(".editor.on-keyboard");
      const from = card?.getBoundingClientRect().bottom;
      // Above the home indicator while the keyboard is down (`.keyboard-down`).
      const margin = () => (card ? Number.parseFloat(getComputedStyle(card).marginBottom) || 0 : 0);
      const restBefore = margin();
      // Now, not with the next render: the card has to start with the keyboard.
      flushSync(update);
      if (card && from !== undefined) {
        const track: Track = {
          kbFrom: clientH - before.inset,
          kbTo: clientH - read().inset,
          gap: CARD_GAP,
          rest: clientH - CARD_GAP - Math.max(restBefore, margin()),
        };
        // A second announcement of the same move finds nothing to move: the
        // first ride's timer stays, and gives the caret back.
        const next = rideCard(card, from, pace, track);
        if (next !== undefined) {
          clearTimeout(timer);
          timer = next;
        }
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
