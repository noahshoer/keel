// The task view's navigation, shared by the whole app (crates/ui/src/task_view/nav.rs
// `TaskViewNav`): its shape and the hook; `TaskNavProvider` (nav.tsx) holds it.
import { createContext, useContext } from "react";
import type { Id, Task } from "../bindings";

/**
 * How long the circles stay held after a tap's call returns: longer than the
 * gap between a double tap's two taps.
 */
export const CIRCLE_SETTLE_MS = 400;

export type TaskNav = {
  /** Open tasks, oldest first. The last one is on screen. */
  stack: readonly Id[];
  /** The view's own show-completed toggle. */
  showCompleted: boolean;
  /** Set when a view should open with its "Add sub-task" editor showing; the view clears it. */
  addSub: boolean;
  /**
   * Set by a tap on the sheet's scrim or a drag that pulls it down: the view
   * saves any pending edit, then closes, and clears it.
   */
  closeAsked: boolean;
  /** Opens a task from a list with its ancestors under it; show-completed starts off. */
  open(id: Id): void;
  /** Opens a task like `open`, with its "Add sub-task" editor open. */
  openAdding(id: Id): void;
  /** Opens a sub-task on top of the current task. */
  push(id: Id): void;
  /** Pops one level; popping the last closes the view. */
  back(): void;
  close(): void;
  /** Asks the open view to close (it saves a pending edit first). */
  askClose(): void;
  setShowCompleted(on: boolean): void;
  clearAddSub(): void;
  clearCloseAsked(): void;
  /** The task on top was completed from its own circle: same as `back`. */
  completedHere(): void;
  /**
   * Holds the view's completion circles for one tap; false when they're
   * already held. A double tap's second tap would otherwise complete the task
   * twice, or land on the circle that took the first one's place (the
   * parent's after a pop, or the next sub-task's).
   */
  holdCircles(): boolean;
  /** Lets the circles take taps again, a moment after the tap's call returned. */
  releaseCircles(): Promise<void>;
  /** Cuts the stack at the first task no longer in `all` (deleted). */
  prune(all: Task[]): void;
};

export const TaskNavContext = createContext<TaskNav | null>(null);

/** The nav from the nearest `TaskNavProvider`. */
export function useTaskNav(): TaskNav {
  const nav = useContext(TaskNavContext);
  if (!nav) throw new Error("useTaskNav outside TaskNavProvider");
  return nav;
}
