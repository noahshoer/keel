// The twelve label colors. A label stores its id; each has a `--label-<id>`
// token in light and dark. "charcoal" is keel-core's default for a new label.
export const LABEL_COLORS: Array<{ id: string; name: string }> = [
  { id: "charcoal", name: "No color" },
  { id: "red", name: "Red" },
  { id: "orange", name: "Orange" },
  { id: "amber", name: "Amber" },
  { id: "green", name: "Green" },
  { id: "teal", name: "Teal" },
  { id: "blue", name: "Blue" },
  { id: "indigo", name: "Indigo" },
  { id: "purple", name: "Purple" },
  { id: "pink", name: "Pink" },
  { id: "brown", name: "Brown" },
  { id: "slate", name: "Slate" },
];

const DEFAULT_ID = "charcoal";

/** The CSS color of a stored label color. Names it doesn't know use the default. */
export function labelColorVar(color: string): string {
  const known = LABEL_COLORS.some((c) => c.id === color);
  return `var(--label-${known ? color : DEFAULT_ID})`;
}

/** A label's chosen color for an icon, or `undefined` when it has none. */
export function labelTagColor(color: string): string | undefined {
  const chosen = labelColorVar(color);
  return chosen === `var(--label-${DEFAULT_ID})` ? undefined : chosen;
}
