// Chip text and classes for a row, every decision from keel-core
// (crates/ui/src/task_row.rs `due_text`, `check_class`, `due_class`).
import type { DueBucket, Priority, Task } from "../bindings";
import { dueBucket, dueDateOf, dueLabel, dueTime, timeLabel } from "../core";

export const CHECK_CLASS: Record<Priority, string> = {
  P1: "check p1",
  P2: "check p2",
  P3: "check p3",
  None: "check",
};

export const FLAG_CLASS: Record<Priority, string | null> = {
  P1: "card-flag p1",
  P2: "card-flag p2",
  P3: "card-flag p3",
  None: null,
};

const DUE_CLASS: Record<DueBucket, string> = {
  Overdue: "due overdue",
  Today: "due today",
  Tomorrow: "due tomorrow",
  ThisWeek: "due week",
  Later: "due later",
};

/** A task's due chip: its colour class and "Friday 5 PM"; `null` without a due. */
export function dueChip(task: Task, today: string): { cls: string; label: string } | null {
  const date = dueDateOf(task);
  if (date === null) return null;
  const time = dueTime(task);
  const day = dueLabel(date, today);
  return { cls: DUE_CLASS[dueBucket(date, today)], label: time === null ? day : `${day} ${timeLabel(time)}` };
}
