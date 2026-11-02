// The task card sits on the keyboard, and its pickers on the card. How far up
// the keyboard reaches is sheet-rules' `keyboardInset`, shared with the task
// view's sheet.
import { useEffect, useState } from "react";
import { keyboardInset } from "../gestures/sheet-rules";
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

export type KeyboardState = { inset: number; up: boolean };

/** Whether the visible area is shorter than the page: the keyboard is up. */
export function keyboardUp(v: VisualViewport): boolean {
  return v.height < document.documentElement.clientHeight - 0.5;
}

function read(): KeyboardState {
  const v = window.visualViewport;
  if (!v) return { inset: 0, up: false };
  const clientH = document.documentElement.clientHeight;
  return { inset: keyboardInset(clientH, v.offsetTop, v.height, null), up: keyboardUp(v) };
}

/** The keyboard's reach while `enabled`, updated as the visual viewport moves. */
export function useKeyboard(enabled: boolean): KeyboardState {
  const [state, setState] = useState<KeyboardState>({ inset: 0, up: false });
  useEffect(() => {
    const v = window.visualViewport;
    if (!enabled || !v) return;
    const update = () => setState(read());
    update();
    v.addEventListener("resize", update);
    v.addEventListener("scroll", update);
    return () => {
      v.removeEventListener("resize", update);
      v.removeEventListener("scroll", update);
    };
  }, [enabled]);
  return state;
}
