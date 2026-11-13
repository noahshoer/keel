// Things that come and go in a list fade and change height instead of
// jumping: the empty view's message (`usePresence`, collapsing through
// `.presence` in app.css) and a card that arrives, a new task or one moved in
// (`useFreshIds` picks it, `growIn` opens it to its own height). A view's
// first render shows what it has without animating; MainPane keys the list by
// view, so a switch starts over.
import { useEffect, useLayoutEffect, useReducer, useRef } from "react";
import { EASE_SOFT } from "../gestures/sheet-rules";
import { prefersReducedMotion } from "../shell/device";

/** How long a collapse or grow takes (`.presence` in app.css matches it). */
export const LEAVE_MS = 280;

type Presence = "out" | "entering" | "shown" | "leaving";
type PresenceEvent = "show" | "hide" | "frame" | "done";

/**
 * In: mounted collapsed ("entering"), opened a frame later ("shown"). Out:
 * collapsed while still mounted ("leaving"), unmounted once that's done.
 */
export function nextPresence(state: Presence, event: PresenceEvent): Presence {
  switch (event) {
    case "show":
      if (state === "out") return "entering";
      return state === "leaving" ? "shown" : state;
    case "hide":
      if (state === "shown") return "leaving";
      return state === "entering" ? "out" : state;
    case "frame":
      return state === "entering" ? "shown" : state;
    case "done":
      return state === "leaving" ? "out" : state;
  }
}

/** Where something shown by `show` is in coming and going; it starts as it is. */
export function usePresence(show: boolean): Presence {
  const [state, send] = useReducer(nextPresence, show ? "shown" : "out");
  useEffect(() => send(show ? "show" : "hide"), [show]);
  // In the render that changes `show`, so it starts with whatever it makes room for.
  useEffect(() => {
    if (state === "entering") {
      const frame = requestAnimationFrame(() => send("frame"));
      return () => cancelAnimationFrame(frame);
    }
    if (state === "leaving") {
      const timer = setTimeout(() => send("done"), LEAVE_MS);
      return () => clearTimeout(timer);
    }
    return undefined;
  }, [state]);
  return nextPresence(state, show ? "show" : "hide");
}

/**
 * The ids that weren't in the last render (none on the first), each fresh
 * until its grow is over, so a render in the meantime doesn't drop it.
 */
export function useFreshIds(ids: readonly string[], now: () => number = Date.now): Set<string> {
  const seen = useRef<Set<string> | null>(null);
  const until = useRef(new Map<string, number>());
  const t = now();
  if (seen.current !== null) {
    for (const id of ids) if (!seen.current.has(id)) until.current.set(id, t + LEAVE_MS * 2);
  }
  seen.current = new Set(ids);
  const fresh = new Set<string>();
  for (const [id, end] of until.current) {
    if (end > t && seen.current.has(id)) fresh.add(id);
    else until.current.delete(id);
  }
  return fresh;
}

/** Opens `el` from nothing to its own height and bottom margin as it fades in (a fade alone under reduced motion). */
export function growIn(el: HTMLElement, reduced: boolean): void {
  if (typeof el.animate !== "function") return;
  if (reduced) {
    el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: LEAVE_MS, easing: EASE_SOFT });
    return;
  }
  const height = el.getBoundingClientRect().height;
  const margin = el.ownerDocument.defaultView?.getComputedStyle(el).marginBottom ?? "0px";
  el.animate(
    [
      { height: "0px", marginBottom: "0px", opacity: 0, overflow: "hidden" },
      { height: `${height}px`, marginBottom: margin, opacity: 1, overflow: "hidden" },
    ],
    { duration: LEAVE_MS, easing: EASE_SOFT },
  );
}

/** Grows `ref`'s element in when it mounts `fresh`. */
export function useGrowIn(ref: React.RefObject<HTMLElement | null>, fresh: boolean): void {
  const first = useRef(fresh);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!first.current || el === null) return;
    const reduced = prefersReducedMotion();
    growIn(el, reduced);
  }, [ref]);
}

/**
 * Records `row`'s height, and its card's, as `--collapse-from`: the switch-off
 * collapses from there (`row-collapse` in app.css), so the rows below rise
 * the whole way instead of jumping at the end.
 */
export function markCollapse(row: HTMLElement): void {
  row.style.setProperty("--collapse-from", `${row.offsetHeight}px`);
  const card = row.closest<HTMLElement>(".task-card");
  card?.style.setProperty("--collapse-from", `${card.offsetHeight}px`);
}
