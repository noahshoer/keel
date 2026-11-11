// What a popup of a row needs from the page: its place against the row, and
// closing on a click or touch outside it, Escape, or any scroll.
import { type RefObject, useEffect, useLayoutEffect } from "react";
import { placeRowMenu } from "./placeRowMenu";

export function useAnchoredPopup({
  anchor,
  popup,
  toggle,
  close,
  layoutKey,
  closeOnScroll = true,
}: {
  /** What it opens against. */
  anchor: RefObject<HTMLElement | null>;
  popup: RefObject<HTMLElement | null>;
  /** The button that opens it, which closes it by itself. */
  toggle?: RefObject<HTMLElement | null> | undefined;
  /** Must keep its identity, or the listeners are put on again each render. */
  close: () => void;
  /** Placed again when this changes (the popup's size does). */
  layoutKey?: unknown;
  /** Whether any scroll closes it (the default). A popup whose content re-flows the page turns it off. */
  closeOnScroll?: boolean;
}) {
  // biome-ignore lint/correctness/useExhaustiveDependencies: `layoutKey` is the trigger, not an input.
  useLayoutEffect(() => {
    const at = anchor.current;
    const pop = popup.current;
    if (!at || !pop) return;
    const vh = window.visualViewport?.height ?? innerHeight;
    const place = placeRowMenu(at.getBoundingClientRect(), pop.getBoundingClientRect(), innerWidth, vh);
    Object.assign(pop.style, {
      position: "fixed",
      right: "auto",
      top: `${place.top}px`,
      left: `${place.left}px`,
    });
  }, [anchor, popup, layoutKey]);

  useEffect(() => {
    const outside = (e: Event) => {
      const t = e.target;
      if (t instanceof Node && !popup.current?.contains(t) && !toggle?.current?.contains(t)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    // A touch, not just a click: a scroll or a swipe that closes the drawer sends no click.
    document.addEventListener("click", outside, true);
    document.addEventListener("contextmenu", outside, true);
    document.addEventListener("touchstart", outside, { capture: true, passive: true });
    document.addEventListener("keydown", onKey);
    if (closeOnScroll) window.addEventListener("scroll", close, true);
    return () => {
      document.removeEventListener("click", outside, true);
      document.removeEventListener("contextmenu", outside, true);
      document.removeEventListener("touchstart", outside, { capture: true });
      document.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", close, true);
    };
  }, [popup, toggle, close, closeOnScroll]);
}
