// The task view's screen rules that aren't about the data: which rows the
// fields card draws once "+ Description" was tapped, when the values it holds
// changed, and how its fire-and-forget saves settle.
import type { EditorValues, Field, FieldLayout } from "../bindings";

/**
 * The rows and pills to draw. "+ Description" opens an empty description row
 * to type into (first, as Description always is), and its pill goes.
 */
export function fieldsShown(layout: FieldLayout, descOpen: boolean): FieldLayout {
  if (!descOpen || layout.fields.includes("Description")) return layout;
  const isDescription = (f: Field) => f === "Description";
  return {
    fields: ["Description", ...layout.fields],
    missing: layout.missing.filter((m) => !(m.missing.kind === "Field" && isDescription(m.missing.field))),
  };
}

/** Whether two sets of values are the same (Rust's `PartialEq` on `EditorValues`). */
export function sameValues(a: EditorValues, b: EditorValues): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** Settles a call, ignoring its failure (the reload shows what landed): true when it went through. */
export const settle = (p: Promise<unknown>): Promise<boolean> =>
  p.then(
    () => true,
    () => false,
  );
