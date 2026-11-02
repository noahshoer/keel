// Where a sidebar row's menu sits: under the row, above it when there's no room
// below, against the row's right edge so it stays inside the drawer.
import { placePopover, type Rect } from "../editor/popover";

const MARGIN = 8;

export function placeRowMenu(row: Rect, menu: Rect, vw: number, vh: number): { top: number; left: number } {
  const { top } = placePopover(row, menu, vw, vh);
  const right = row.left + row.width;
  return { top, left: Math.max(MARGIN, Math.min(right - menu.width, vw - MARGIN - menu.width)) };
}
