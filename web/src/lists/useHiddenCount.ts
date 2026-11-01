// "+N" for labels cut off at the end of a row, measured from the DOM and again
// whenever the row's width changes.
import { type RefObject, useLayoutEffect, useState } from "react";
import { hiddenCount } from "./rowLayout";

export function useHiddenCount(ref: RefObject<HTMLElement | null>, key: string): number {
  const [count, setCount] = useState(0);
  // biome-ignore lint/correctness/useExhaustiveDependencies: `key` is what's rendered inside the measured element.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => {
      const box = el.getBoundingClientRect();
      if (box.width <= 0) return;
      const lefts = [...el.querySelectorAll("[data-label]")].map(
        (l) => l.getBoundingClientRect().left - box.left,
      );
      setCount(hiddenCount(lefts, box.width));
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref, key]);
  return count;
}
