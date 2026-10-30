// keel-core's data rules in the browser (crates/core-wasm). The only module
// that imports ./pkg (Biome noRestrictedImports): everything else calls these
// wrappers, which refuse to run until initCore() has loaded the wasm.
import type {
  Card,
  CardMode,
  DateGroup,
  DateGroupMode,
  DueBucket,
  Id,
  Progress,
  Reminder,
  Task,
} from "../bindings";
import init, * as core from "./pkg/keel_core_wasm.js";

let initialised = false;
let loading: Promise<void> | null = null;

/** Loads the wasm; await it before mounting the app. Concurrent calls share one load. */
export function initCore(): Promise<void> {
  if (initialised) return Promise.resolve();
  loading ??= init().then(() => {
    initialised = true;
  });
  return loading;
}

/** Loads the wasm from its bytes, synchronously (tests: src/test/wasm.ts). */
export function initCoreSync(bytes: BufferSource): void {
  if (initialised) return;
  core.initSync({ module: bytes });
  initialised = true;
}

function ready(): void {
  if (!initialised) throw new Error("keel core not initialised: await initCore() first");
}

/** Due chip text ("Today", "Friday", "3 Nov"). Both dates are `YYYY-MM-DD`. */
export function dueLabel(due: string, today: string): string {
  ready();
  return core.dueLabel(due, today);
}

/** The calendar date (`YYYY-MM-DD`) a task's due falls on; `null` without a due. */
export function dueDateOf(task: Task): string | null {
  ready();
  return core.dueDateOf(task);
}

/** The time of day (`HH:MM:SS`) of a task's due; `null` for a date-only due or none. */
export function dueTime(task: Task): string | null {
  ready();
  return core.dueTime(task);
}

/** 12-hour label ("5 PM", "5:30 PM") for an `HH:MM[:SS]` time. */
export function timeLabel(time: string): string {
  ready();
  return core.timeLabel(time);
}

/** A parent's direct sub-task counts; `null` when nothing is counted. */
export function progress(parent: Id, all: Task[]): Progress | null {
  ready();
  return core.progress(parent, all);
}

/** Groups a view's matched tasks into cards (sub-tasks under their parent). */
export function cards(matched: Task[], all: Task[], mode: CardMode): Card[] {
  ready();
  return core.cards(matched, all, mode);
}

/** Which coloured due chip a `YYYY-MM-DD` due gets relative to `today`. */
export function dueBucket(due: string, today: string): DueBucket {
  ready();
  return core.dueBucket(due, today);
}

/** True when there is at least one counted sub-task and all are checked. */
export function allDone(p: Progress): boolean {
  ready();
  return core.allDone(p);
}

/** Today / Upcoming split into titled date groups (Overdue first). */
export function dateGroups(mode: DateGroupMode, tasks: Task[], today: string): DateGroup[] {
  ready();
  return core.dateGroups(mode, tasks, today);
}

/** The Upcoming view's tasks: the overdue ones from Today's, then the window. */
export function upcomingTasks(todayTasks: Task[], window: Task[], today: string): Task[] {
  ready();
  return core.upcomingTasks(todayTasks, window, today);
}

/** A deadline (`YYYY-MM-DD`) is urgent on the day and after. */
export function deadlineUrgent(deadline: string, today: string): boolean {
  ready();
  return core.deadlineUrgent(deadline, today);
}

/** A reminder's text: "30 min before", "Tuesday 9 AM". */
export function reminderLabel(r: Reminder, today: string): string {
  ready();
  return core.reminderLabel(r, today);
}
