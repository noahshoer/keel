// Where an open picker sits: next to its chip, inside the viewport. Below
// the chip, flipped above when there's no room, shifted left at the right
// edge. Fixed positioning, so scrolling containers never clip it (the
// Dioxus `PLACE_JS` in crates/ui/src/pickers.rs).
import { type RefObject, useLayoutEffect } from "react";
import { placeAboveCard } from "./keyboardAnchor";

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

/** The top of what's visible, below the status bar, from the visible top. */
function visibleTop(): number {
  const safe = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--safe-top"));
  return (Number.isNaN(safe) ? 0 : safe) + MARGIN;
}

/**
 * Keeps a picker above the task card as the keyboard moves or its content
 * changes. Rects are relative to what's visible; a fixed `top` is relative to
 * the layout viewport, which starts `offsetTop` higher once WebKit scrolls.
 */
function followCard(pop: HTMLElement, card: Element): () => void {
  const place = () => {
    Object.assign(pop.style, { position: "fixed", right: "auto", margin: "0", maxHeight: "" });
    const at = placeAboveCard(
      card.getBoundingClientRect(),
      pop.getBoundingClientRect(),
      innerWidth,
      visibleTop(),
    );
    const offset = window.visualViewport?.offsetTop ?? 0;
    Object.assign(pop.style, {
      top: `${at.top + offset}px`,
      left: `${at.left}px`,
      maxHeight: `${at.maxHeight}px`,
      overflowY: "auto",
    });
  };
  place();
  const v = window.visualViewport;
  v?.addEventListener("resize", place);
  v?.addEventListener("scroll", place);
  const resized = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(place);
  resized?.observe(card);
  // The card moves after the viewport does, when its inset state lands.
  const moved = new MutationObserver(place);
  moved.observe(card, { attributes: true, attributeFilter: ["style", "class"] });
  // A step inside the picker swaps its content.
  const changed = new MutationObserver(place);
  changed.observe(pop, { childList: true, subtree: true });
  return () => {
    v?.removeEventListener("resize", place);
    v?.removeEventListener("scroll", place);
    resized?.disconnect();
    moved.disconnect();
    changed.disconnect();
  };
}

/**
 * Places the popover when it opens: above the task card when that sits on the
 * keyboard, otherwise against the element before it (its chip).
 */
export function usePlacedPopover(ref: RefObject<HTMLElement | null>): void {
  useLayoutEffect(() => {
    const pop = ref.current;
    const card = pop?.closest(".on-keyboard");
    if (pop && card) return followCard(pop, card);
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
