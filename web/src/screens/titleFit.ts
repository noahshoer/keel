// A view title holds to two lines and never breaks inside a word: it shrinks
// until it fits, then cuts between words. The browser does the wrapping, so
// `useTitleFit` measures the heading itself at each candidate.
import { type RefObject, useLayoutEffect, useRef, useState } from "react";

const MIN_SIZE = 22;
const FLOOR_SIZE = 12;
const STEP = 2;
const LINES = 2;

export type TitleFit = { size: number; text: string };

export function fitTitle(
  name: string,
  start: number,
  fits: (size: number, text: string) => boolean,
): TitleFit {
  for (let size = start; size >= MIN_SIZE; size -= STEP) {
    if (fits(size, name)) return { size, text: name };
  }
  const words = name.split(/\s+/);
  for (let keep = words.length - 1; keep >= 1; keep -= 1) {
    const text = `${words.slice(0, keep).join(" ")}…`;
    if (fits(MIN_SIZE, text)) return { size: MIN_SIZE, text };
  }
  // One word wider than the line: only a smaller size keeps it whole.
  const text = words.length > 1 ? `${words[0]}…` : name;
  for (let size = MIN_SIZE - STEP; size > FLOOR_SIZE; size -= STEP) {
    if (fits(size, text)) return { size, text };
  }
  return { size: FLOOR_SIZE, text };
}

/** Measures with the heading's own styles, and leaves it as React last drew it. */
function measure(el: HTMLElement, name: string): (TitleFit & { lineHeight: number }) | null {
  const drawn = el.textContent;
  const { fontSize, lineHeight } = el.style;
  el.style.fontSize = "";
  el.style.lineHeight = "";
  const css = getComputedStyle(el);
  const start = Number.parseFloat(css.fontSize);
  const ratio = Number.parseFloat(css.lineHeight) / start;
  let fit: (TitleFit & { lineHeight: number }) | null = null;
  if (start > 0 && ratio > 0) {
    const best = fitTitle(name, start, (size, text) => {
      el.textContent = text;
      el.style.fontSize = `${size}px`;
      el.style.lineHeight = `${size * ratio}px`;
      return el.scrollWidth <= el.clientWidth && el.scrollHeight <= size * ratio * LINES + 1;
    });
    fit = { ...best, lineHeight: best.size * ratio };
  }
  el.textContent = drawn;
  el.style.fontSize = fontSize;
  el.style.lineHeight = lineHeight;
  return fit;
}

export function useTitleFit(name: string): {
  ref: RefObject<HTMLHeadingElement | null>;
  text: string;
  style: { fontSize: number; lineHeight: string } | undefined;
} {
  const ref = useRef<HTMLHeadingElement>(null);
  const [fit, setFit] = useState<(TitleFit & { lineHeight: number; name: string }) | null>(null);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    let live = true;
    const run = () => {
      if (!live) return;
      const next = measure(el, name);
      setFit(next ? { ...next, name } : null);
    };
    run();
    // Fonts arriving and rotation both change what fits. Only width counts:
    // the height moves with every fit.
    document.fonts?.ready.then(run);
    let width = el.clientWidth;
    const observer =
      typeof ResizeObserver === "function"
        ? new ResizeObserver(() => {
            if (el.clientWidth === width) return;
            width = el.clientWidth;
            run();
          })
        : null;
    observer?.observe(el);
    return () => {
      live = false;
      observer?.disconnect();
    };
  }, [name]);

  const current = fit?.name === name ? fit : null;
  return {
    ref,
    text: current?.text ?? name,
    style: current ? { fontSize: current.size, lineHeight: `${current.lineHeight}px` } : undefined,
  };
}
