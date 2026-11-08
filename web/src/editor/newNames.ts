// The save prompt's ticks: which of the new `#project` / `@label` names to
// create with the task (screen rules; the names come from keel-core's
// `newNames`, and the unticked ones go back to it through `keepAsText`).
import type { NewNames } from "../bindings";

export type Ticks = { projects: Record<string, boolean>; labels: Record<string, boolean> };

const ticked = (names: string[]) => Object.fromEntries(names.map((n) => [n, true]));

/** Every name ticked, as the prompt opens. */
export function allTicked(names: NewNames): Ticks {
  return { projects: ticked(names.projects), labels: ticked(names.labels) };
}

/** The header check of a group: all, some or none of its names ticked. */
export function groupState(ticks: Record<string, boolean>): "all" | "some" | "none" {
  const values = Object.values(ticks);
  const on = values.filter(Boolean).length;
  if (on === values.length) return "all";
  return on === 0 ? "none" : "some";
}

/** The header's tap: all ticked → all off, otherwise all on. */
export function toggleGroup(ticks: Record<string, boolean>): Record<string, boolean> {
  const on = groupState(ticks) !== "all";
  return Object.fromEntries(Object.keys(ticks).map((n) => [n, on]));
}

const off = (ticks: Record<string, boolean>) =>
  Object.entries(ticks)
    .filter(([, on]) => !on)
    .map(([n]) => n);

/** The unticked names, by kind: what stays text in the title. */
export function unticked(t: Ticks): NewNames {
  return { projects: off(t.projects), labels: off(t.labels) };
}

/** "1 of 2": how many of a group's names are ticked. */
export function countText(ticks: Record<string, boolean>): string {
  const values = Object.values(ticks);
  return `${values.filter(Boolean).length} of ${values.length}`;
}
