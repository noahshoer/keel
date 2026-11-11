/**
 * How many px of the layout viewport's bottom the keyboard covers: what's left
 * of the layout height under the visual viewport (its height and how far it has
 * scrolled down).
 */
export function keyboardInset(
  layoutHeight: number,
  viewportHeight: number,
  viewportOffsetTop: number,
): number {
  return Math.max(0, Math.round(layoutHeight - viewportHeight - viewportOffsetTop));
}
