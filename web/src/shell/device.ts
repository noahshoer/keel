// What the device and the window say about themselves: the one place the
// reduced-motion, width and pointer media queries are named.

/** The desktop layout, and the task view's panel instead of its sheet, start at this width. */
export const WIDE_MIN = 900;
export const WIDE_QUERY = `(min-width: ${WIDE_MIN}px)`;
/** The width below `WIDE_MIN` (the CSS `max-width: 899px` breakpoint). */
export const NARROW_QUERY = `(max-width: ${WIDE_MIN - 1}px)`;

/** Whether the person asked for less motion. False where `matchMedia` is missing. */
export function prefersReducedMotion(win: Pick<Window, "matchMedia"> | null | undefined = window): boolean {
  return (
    typeof win?.matchMedia === "function" &&
    win.matchMedia("(prefers-reduced-motion: reduce)").matches === true
  );
}

/** True on touch screens (phones), where the task card sits on the keyboard and time is a wheel. */
export function isTouch(): boolean {
  return typeof matchMedia === "function" && matchMedia("(pointer: coarse)").matches;
}
