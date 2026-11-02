// Where a sideways row of pills fades: on whichever side one runs past.
import { useCallback, useLayoutEffect, useRef, useState } from "react";

/** Whether a pill still runs past the end of the row. */
export function pillsRunOn(scrollLeft: number, clientWidth: number, scrollWidth: number): boolean {
  return scrollLeft + clientWidth < scrollWidth - 1;
}

/** Whether a pill has scrolled past the start of the row. */
export function pillsRunBack(scrollLeft: number): boolean {
  return scrollLeft > 1;
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
