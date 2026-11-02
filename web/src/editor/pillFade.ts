// Where a sideways row of pills fades: on whichever side one runs past. And
// which pill it scrolls to, when a phrase typed in the title fills one.
import { type RefObject, useCallback, useLayoutEffect, useRef, useState } from "react";

/** Whether a pill still runs past the end of the row. */
export function pillsRunOn(scrollLeft: number, clientWidth: number, scrollWidth: number): boolean {
  return scrollLeft + clientWidth < scrollWidth - 1;
}

/** Whether a pill has scrolled past the start of the row. */
export function pillsRunBack(scrollLeft: number): boolean {
  return scrollLeft > 1;
}

/** What the pills with a typed phrase say, by pill, in the order they were typed. */
export type TypedPills = Record<string, string>;

/** The pill a phrase just filled or changed, if any: the last, when several did. */
export function filledPill(before: TypedPills, after: TypedPills): string | null {
  return (
    Object.keys(after)
      .reverse()
      .find((pill) => after[pill] !== before[pill]) ?? null
  );
}

/**
 * Where a row scrolls to show a pill, given the part of the row that's in view
 * (`from` to `to`) and the pill's edges: no further than it has to. A pill too
 * wide for the room shows its start.
 */
export function revealLeft(
  scrollLeft: number,
  from: number,
  to: number,
  pillLeft: number,
  pillRight: number,
): number {
  if (pillLeft < from || pillRight - pillLeft > to - from) return scrollLeft + pillLeft - from;
  if (pillRight > to) return scrollLeft + pillRight - to;
  return scrollLeft;
}

/** The space between the last pill in view and the save button. */
const SAVE_GAP = 8;

/**
 * Scrolls the row beside a save button to the pill a typed phrase just filled,
 * so what was understood shows. Pills are found by their `title`. Picks on the
 * pills and phrases going never move it. Without a save button (the pills wrap)
 * it does nothing.
 */
export function useRevealFilled(row: RefObject<HTMLElement | null>, typed: TypedPills): void {
  const before = useRef(typed);
  useLayoutEffect(() => {
    const pill = filledPill(before.current, typed);
    before.current = typed;
    const el = row.current;
    const save = el?.parentElement?.querySelector(".btn-save");
    const button = pill === null ? null : el?.querySelector(`[title="${pill}"]`);
    if (!el || !save || !button) return;
    const inset = Number.parseFloat(getComputedStyle(el).paddingLeft) || 0;
    const { left, right } = button.getBoundingClientRect();
    el.scrollLeft = revealLeft(
      el.scrollLeft,
      el.getBoundingClientRect().left + inset,
      save.getBoundingClientRect().left - SAVE_GAP,
      left,
      right,
    );
  });
}

/**
 * For a row of pills: its `ref` and `onScroll`, and the classes for an element
 * around it that draws the fades. `content` is what changes the pills' widths.
 */
export function usePillFade(content: unknown) {
  const ref = useRef<HTMLDivElement>(null);
  const [on, setOn] = useState(false);
  const [back, setBack] = useState(false);
  const onScroll = useCallback(() => {
    const row = ref.current;
    if (!row) return;
    setOn(pillsRunOn(row.scrollLeft, row.clientWidth, row.scrollWidth));
    setBack(pillsRunBack(row.scrollLeft));
  }, []);
  // biome-ignore lint/correctness/useExhaustiveDependencies: `content` changes the pills' widths.
  useLayoutEffect(onScroll, [onScroll, content]);
  // `content` too: the row can appear after the first draw.
  // biome-ignore lint/correctness/useExhaustiveDependencies: see above.
  useLayoutEffect(() => {
    const row = ref.current;
    if (!row || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(onScroll);
    observer.observe(row);
    return () => observer.disconnect();
  }, [onScroll, content]);
  // Pills change width once the app's font has loaded.
  useLayoutEffect(() => {
    let live = true;
    void document.fonts?.ready.then(() => {
      if (live) onScroll();
    });
    return () => {
      live = false;
    };
  }, [onScroll]);
  const fadeClass = [on && "pills-run-on", back && "pills-run-back"].filter(Boolean).join(" ");
  return { ref, onScroll, fadeClass };
}
