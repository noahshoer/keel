// A form in the drawer stays above the keyboard. While the drawer is open the
// page holds still (App's `holdPage`), so the drawer is the one thing that
// scrolls: it gains the keyboard's height as room to scroll into, and moves the
// whole form above the keyboard, where WebKit alone would reveal only the cursor.
import { type RefObject, useEffect, useState } from "react";
import { useKeyboard } from "../editor/keyboardAnchor";
import { isTouch } from "../editor/locale";

/** Keeps the form clear of the keyboard's top edge. */
const MARGIN = 12;
/**
 * Scroll room past the form, so it can shrink (the filter hint giving way to a
 * shorter error) without the drawer's scroll limit pulling everything down.
 */
const SPARE = 80;

type Span = { top: number; bottom: number };

/** The drawer's `scrollTop` that brings `form` into `room`, its top first if it doesn't fit. */
export function revealedScrollTop(scrollTop: number, form: Span, room: Span): number {
  if (form.top < room.top) return scrollTop - (room.top - form.top);
  const under = form.bottom - room.bottom;
  if (under <= 0) return scrollTop;
  return scrollTop + Math.min(under, form.top - room.top);
}

/** The room the drawer needs below its rows for the keyboard (0 when it's down). */
export function useDrawerKeyboard(sidebar: RefObject<HTMLElement | null>): number {
  const [touch] = useState(isTouch);
  const { up, inset, room } = useKeyboard(touch);
  useEffect(() => {
    const drawer = sidebar.current;
    if (!drawer || !up) return;
    const reveal = (behavior: ScrollBehavior) => {
      const form = document.activeElement?.closest("form");
      if (!form || !drawer.contains(form)) return;
      const top = revealedScrollTop(drawer.scrollTop, form.getBoundingClientRect(), {
        top: Number.parseFloat(getComputedStyle(drawer).paddingTop),
        bottom: room - MARGIN,
      });
      drawer.scrollTo({ top, behavior });
    };
    const onFocus = () => reveal("smooth");
    // Typing into a field scrolled out of view brings it back. WebKit reveals
    // the cursor after the keystroke, which would cancel an animated scroll.
    const onInput = () => requestAnimationFrame(() => requestAnimationFrame(() => reveal("auto")));
    onFocus();
    drawer.addEventListener("focusin", onFocus);
    drawer.addEventListener("input", onInput);
    return () => {
      drawer.removeEventListener("focusin", onFocus);
      drawer.removeEventListener("input", onInput);
    };
  }, [sidebar, up, room]);
  return up ? inset + SPARE : 0;
}
