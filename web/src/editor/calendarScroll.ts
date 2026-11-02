// Where the scrolling calendar is. All weeks are one height, so positions come
// from week numbers, weeks far from the view aren't drawn, and adding them never
// moves the scroll (which would stop a flick on iOS).

/** The window moves in stretches this long, so most scrolling redraws nothing. */
const STRETCH = 8;

/**
 * The month to show: the one at the view's middle, row 3 of the six in view. A
 * month has at least four weeks, so settled on its 1st it's still that month,
 * and the next one takes over as soon as it fills the lower half. Snapping can
 * settle a fraction of a pixel short.
 */
export function monthInView(tops: number[], scrollTop: number, rowHeight: number): number {
  const at = scrollTop + rowHeight * 3 + 0.5;
  let lo = 0;
  let hi = tops.length - 1;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if ((tops[mid] ?? 0) <= at) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

/** The weeks to draw, `[from, to)`, with `topWeek` at the top of the view, of `total`. */
export function weekWindow(topWeek: number, total: number): [number, number] {
  const base = Math.floor(topWeek / STRETCH) * STRETCH;
  return [Math.max(0, base - STRETCH), Math.min(total, base + STRETCH * 3)];
}
