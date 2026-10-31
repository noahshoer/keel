// Navigation for the task view: a stack of open tasks, oldest first. The last
// one is on screen. Immutable; each step returns the next stack.
import type { Id, Task } from "../bindings";
import { chain } from "../core";

export type ViewStack = readonly Id[];

/** `id` on top of its live ancestors, top-level first, so Back walks up to the parent. */
export function openChain(id: Id, all: Task[]): Id[] {
  return chain(id, all);
}

/** Opens a sub-task on top of the current task. */
export function push(s: ViewStack, id: Id): Id[] {
  return [...s, id];
}

/** Pops one level; popping the last closes the view. */
export function back(s: ViewStack): Id[] {
  return s.slice(0, -1);
}

/**
 * Cuts the stack at the first id not in `all`. A deleted task's sub-tasks are
 * deleted with it, so everything above it goes too. Returns `s` itself when
 * nothing was cut.
 */
export function prune(s: ViewStack, all: Task[]): ViewStack {
  const live = new Set(all.map((t) => t.id));
  const keep = s.findIndex((id) => !live.has(id));
  return keep === -1 ? s : s.slice(0, keep);
}

export function top(s: ViewStack): Id | null {
  return s.at(-1) ?? null;
}

/** The task below the top one. */
export function under(s: ViewStack): Id | null {
  return s.at(-2) ?? null;
}
