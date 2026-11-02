// Where each month sits in the scrolling calendar. Heights come from the row
// counts, so months far from the view aren't drawn and adding them never moves
// the scroll (which would stop a flick on iOS).

/** Months drawn on each side of the shown one. */
const RADIUS = 3;

/** Each month's top, given its row count. */
export function monthTops(rows: number[], rowHeight: number, gap: number): number[] {
  let top = 0;
  return rows.map((n) => {
    const at = top;
    top += n * rowHeight + gap;
    return at;
  });
}

/** The month at the top of the view; snapping can settle a fraction of a pixel short. */
export function monthAt(tops: number[], scrollTop: number): number {
  const at = scrollTop + 0.5;
  let lo = 0;
  let hi = tops.length - 1;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if ((tops[mid] ?? 0) <= at) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

/** The first and last month to draw around `index`, of `count`. */
export function monthWindow(index: number, count: number): [number, number] {
  return [Math.max(0, index - RADIUS), Math.min(count - 1, index + RADIUS)];
}
