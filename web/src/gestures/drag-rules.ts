// What the drag of a task decides, apart from the page: when a press has moved
// enough to be a drag, how fast the page scrolls under a finger near its edge,
// and whether a drop is a move at all (`drag.ts` uses them).

/** A touch that has moved this far (px) since the hold is a drag. */
export const TOUCH_SLOP = 10;
/** A mouse press that has moved this far is a drag (a click moves a little). */
export const MOUSE_SLOP = 6;
/** The page scrolls while the finger is within this many px of its top or bottom. */
export const EDGE_ZONE = 72;
/** The fastest the page scrolls under a drag (px per frame). */
export const MAX_SCROLL = 18;

export function movedBeyond(dx: number, dy: number, slop: number): boolean {
  return Math.hypot(dx, dy) > slop;
}

/**
 * How far the page scrolls this frame for a finger at `y` over a list that
 * spans `top` to `bottom`: negative up, positive down, zero away from the
 * edges; faster the nearer (or past) the edge.
 */
export function edgeScroll(y: number, top: number, bottom: number): number {
  const into = (depth: number) => Math.min(Math.max(depth / EDGE_ZONE, 0), 1) * MAX_SCROLL;
  if (y < top + EDGE_ZONE) return -Math.round(into(top + EDGE_ZONE - y));
  if (y > bottom - EDGE_ZONE) return Math.round(into(y - (bottom - EDGE_ZONE)));
  return 0;
}

/** The section of a zone that starts a new one: no task is there already. */
export const NEW_SECTION = "+new";

/** Where a task is, or where a drop zone puts one: a project, and a section ("" for none). */
export type Place = { project: string; section: string };

/** Dropping a task where it already is does nothing. */
export function isMove(from: Place, to: Place): boolean {
  return from.project !== to.project || from.section !== to.section;
}
