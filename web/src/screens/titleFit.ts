// A view title holds to one line and, when it can, cuts between words. The
// browser does the measuring, so `useTitleFit` tries each cut on the heading
// itself. A first word wider than the line is left to `text-overflow`.
import { type RefObject, useLayoutEffect, useRef, useState } from "react";

export function fitTitle(name: string, fits: (text: string) => boolean): string {
  if (fits(name)) return name;
  const words = name.split(/\s+/);
  for (let keep = words.length - 1; keep >= 1; keep -= 1) {
    const text = `${words.slice(0, keep).join(" ")}…`;
    if (fits(text)) return text;
  }
  return name;
}

/** Measures with the heading's own styles, and leaves it as React last drew it. */
function measure(el: HTMLElement, name: string): string {
  const drawn = el.textContent;
  const text = fitTitle(name, (candidate) => {
    el.textContent = candidate;
    return el.scrollWidth <= el.clientWidth;
  });
  el.textContent = drawn;
  return text;
}

export function useTitleFit(name: string): {
  ref: RefObject<HTMLHeadingElement | null>;
  text: string;
} {
  const ref = useRef<HTMLHeadingElement>(null);
  const [fit, setFit] = useState<{ name: string; text: string } | null>(null);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    let live = true;
    const run = () => {
      if (live) setFit({ name, text: measure(el, name) });
    };
    run();
    // Fonts arriving and rotation both change what fits.
    document.fonts?.ready.then(run);
    const observer = typeof ResizeObserver === "function" ? new ResizeObserver(run) : null;
    observer?.observe(el);
    return () => {
      live = false;
      observer?.disconnect();
    };
  }, [name]);

  return { ref, text: fit?.name === name ? fit.text : name };
}
