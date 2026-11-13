// keel-core's data rules in the browser (crates/core-wasm). The only module
// that imports ./pkg (Biome noRestrictedImports). The wasm functions and their
// doc comments are generated from the Rust (see pkg/keel_core_wasm.d.ts), so
// they are re-exported as they are; only the functions that reshape an
// argument or the result are written out here. Call initCore() (src/main.tsx)
// before any of them; before that they fail inside the wasm glue.
import type { DayCell, Label, Project, Recurrence, Section, Task } from "../bindings";
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

export {
  addPlan,
  allDone,
  applyPick,
  applyTitle,
  arrange,
  arrangementFlags,
  arrangementSummary,
  blankValues,
  cards,
  chain,
  customReminderAt,
  dateGroups,
  daySubtitle,
  deadlineUrgent,
  dueBucket,
  dueDateOf,
  dueLabel,
  dueTime,
  editorView,
  editPlan,
  fieldLayout,
  filterReading,
  importLine,
  keepAsText,
  labelChoices,
  labelChosen,
  labelGroups,
  locationChoices,
  locationPick,
  locationText,
  monthFirstWeeks,
  monthNames,
  monthView,
  movesOccurrence,
  nativeTime,
  newNames,
  pickSetting,
  progress,
  progressLong,
  progressShort,
  quickDates,
  readRepeat,
  reminderActions,
  reminderDraftLabel,
  reminderDraftOf,
  reminderLabel,
  reminderMenus,
  reminderPresets,
  repeatText,
  shiftMonth,
  sidebarCounts,
  sortChoices,
  streakText,
  taskDetail,
  taskWithProject,
  timeLabel,
  toggleLabel,
  typedTime,
  unhighlightAt,
  upcomingTasks,
  upcomingWhen,
  valuesFromTask,
  withProject,
  withStoredPlain,
  withTitle,
  yearChoices,
} from "./pkg/keel_core_wasm.js";

/** The filter query with the term ending at `caret` (UTF-16) quoted; `null` when none ends there. */
export function filterUnhighlightAt(
  query: string,
  projects: Project[],
  sections: Section[],
  labels: Label[],
  today: string,
  caret: number,
): string | null {
  return core.filterUnhighlightAt(query, projects, sections, labels, today, caret) ?? null;
}

/** "Moved from Wed 21 Oct" while a repeating task's date stands in for another occurrence, else null. */
export function movedNote(task: Task): string | null {
  return core.movedNote(task) ?? null;
}

/** `count` weeks of the calendar's run of weeks, from week `from` as `monthFirstWeeks` counts them; `shown` is [year, month]. */
export function weekRows(
  firstYear: number,
  weekStart: number,
  from: number,
  count: number,
  shown: [number, number],
  today: string,
  selected: string | null,
  rule: Recurrence | null,
): DayCell[][] {
  return core.weekRows(firstYear, weekStart, from, count, shown[0], shown[1], today, selected, rule);
}
