// Where an open picker sits: next to its chip, inside the viewport. Below
// the chip, flipped above when there's no room, shifted left at the right
// edge. Fixed positioning, so scrolling containers never clip it (the
// Dioxus `PLACE_JS` in crates/ui/src/pickers.rs).
import { type RefObject, useLayoutEffect } from "react";

export type Rect = { top: number; bottom: number; left: number; width: number; height: number };

const MARGIN = 8;
const GAP = 6;

export function placePopover(chip: Rect, pop: Rect, vw: number, vh: number): { top: number; left: number } {
  let top = chip.bottom + GAP;
  if (top + pop.height > vh - MARGIN && chip.top - GAP - pop.height >= MARGIN)
    top = chip.top - GAP - pop.height;
  top = Math.max(MARGIN, Math.min(top, vh - MARGIN - pop.height));
  const left = Math.max(MARGIN, Math.min(chip.left, vw - MARGIN - pop.width));
  return { top, left };
}

/** Places the popover against the element before it (its chip) when it opens. */
export function usePlacedPopover(ref: RefObject<HTMLElement | null>): void {
  useLayoutEffect(() => {
    const pop = ref.current;
    const chip = pop?.previousElementSibling;
    if (!pop || !chip) return;
    const { top, left } = placePopover(
      chip.getBoundingClientRect(),
      pop.getBoundingClientRect(),
      innerWidth,
      innerHeight,
    );
    Object.assign(pop.style, {
      position: "fixed",
      top: `${top}px`,
      left: `${left}px`,
      right: "auto",
      margin: "0",
    });
  }, [ref]);
}
