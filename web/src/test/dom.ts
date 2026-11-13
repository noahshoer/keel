import type { Rect } from "../editor/popover";

/** A bounding rect from its corner and size. */
export const rect = (left: number, top: number, width: number, height: number): Rect => ({
  left,
  top,
  width,
  height,
  bottom: top + height,
});
