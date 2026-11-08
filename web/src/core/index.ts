// keel-core's data rules in the browser (crates/core-wasm). The only module
// that imports ./pkg (Biome noRestrictedImports): everything else calls these
// wrappers, which refuse to run until initCore() has loaded the wasm.
import type {
  Action,
  AddPlan,
  Card,
  CardMode,
  DateGroup,
  DateGroupMode,
  DayCell,
  DueBucket,
  EditorPick,
  EditorValues,
  EditorView,
  EditPlan,
  FieldLayout,
  Id,
  ImportSummary,
  Label,
  LabelChoice,
  LabelGroup,
  LocationChoice,
  MenuKind,
  MonthView,
  NewNames,
  Progress,
  Project,
  Recurrence,
  Reminder,
  ReminderDraft,
  ReminderMenus,
  ReminderSettings,
  RepeatReading,
  Section,
  SidebarCounts,
  Task,
  TaskDetail,
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
export function progress(parent: Id, all: Task[], today: string): Progress | null {
  ready();
  return core.progress(parent, all, today);
}

/** Groups a view's matched tasks into cards (sub-tasks under their parent). */
export function cards(matched: Task[], all: Task[], mode: CardMode, today: string): Card[] {
  ready();
  return core.cards(matched, all, mode, today);
}

/** Which coloured due chip a `YYYY-MM-DD` due gets relative to `today`. */
export function dueBucket(due: string, today: string): DueBucket {
  ready();
  return core.dueBucket(due, today);
}

/** A streak in words, in its repeat's unit: "4 day streak" or "Streak of 4". */
export function streakText(task: Task): string {
  ready();
  return core.streakText(task);
}

/** The numbers beside the sidebar's rows: how many tasks each view lists. */
export function sidebarCounts(
  all: Task[],
  today: string,
  upcomingDays: number,
  showSubtasks: boolean,
): SidebarCounts {
  ready();
  return core.sidebarCounts(all, today, upcomingDays, showSubtasks);
}

/** The labels picker's rows: those matching `query`, each marked if chosen. */
export function labelChoices(labels: Label[], selected: string[], query: string): LabelChoice[] {
  ready();
  return core.labelChoices(labels, selected, query);
}

/** Whether `name` is among the chosen labels. */
export function labelChosen(selected: string[], name: string): boolean {
  ready();
  return core.labelChosen(selected, name);
}

/** A task's labels grouped by color, in added order. */
export function labelGroups(taskLabels: string[], labels: Label[]): LabelGroup[] {
  ready();
  return core.labelGroups(taskLabels, labels);
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

/** The Today view's subtitle for a `YYYY-MM-DD` date: "Friday 30 Oct". */
export function daySubtitle(today: string): string {
  ready();
  return core.daySubtitle(today);
}

/** A parent's sub-task count as "2/5". */
export function progressShort(p: Progress): string {
  ready();
  return core.progressShort(p);
}

/** A parent's sub-task count as "2 of 5". */
export function progressLong(p: Progress): string {
  ready();
  return core.progressLong(p);
}

// The task view (keel_core::task_view).

/** The fields card: set fields as rows, missing ones as pills. */
export function fieldLayout(v: EditorValues, inboxId: Id): FieldLayout {
  ready();
  return core.fieldLayout(v, inboxId);
}

/** What one level of the task view renders; null for an unknown id. */
export function taskDetail(id: Id, all: Task[], showCompleted: boolean, today: string): TaskDetail | null {
  ready();
  return core.taskDetail(id, all, showCompleted, today);
}

/** A task's ancestors from the top, then the task itself. */
export function chain(id: Id, all: Task[]): Id[] {
  ready();
  return core.chain(id, all);
}

// The task editor (keel_core::editor). Every function takes and returns
// whole values; the editor keeps them in state and re-renders from editorView.

/** Fresh editor values for adding into a project (and section) with defaults. */
export function blankValues(
  projectId: Id,
  sectionId: Id | null,
  date: string,
  labels: string[],
): EditorValues {
  ready();
  return core.blankValues(projectId, sectionId, date, labels);
}

/** The editor values for an existing task. */
export function valuesFromTask(t: Task): EditorValues {
  ready();
  return core.valuesFromTask(t);
}

/** Everything the editor draws for its values, from one parse of the title. */
export function editorView(
  v: EditorValues,
  quickAdd: boolean,
  keepLabel: string | null,
  existing: Recurrence | null,
  projects: Project[],
  sections: Section[],
  labels: Label[],
  today: string,
): EditorView {
  ready();
  return core.editorView(v, quickAdd, keepLabel, existing, projects, sections, labels, today);
}

/** The values after the title is retyped (kept-plain phrases follow the text). */
export function withTitle(v: EditorValues, text: string): EditorValues {
  ready();
  return core.withTitle(v, text);
}

/** Applies a picker's value; with quick add, the typed phrase it replaces leaves the title. */
export function applyPick(
  v: EditorValues,
  pick: EditorPick,
  quickAdd: boolean,
  projects: Project[],
  sections: Section[],
  today: string,
): EditorValues {
  ready();
  return core.applyPick(v, pick, quickAdd, projects, sections, today);
}

/** Ticks or unticks one label (typed @labels count as ticked). */
export function toggleLabel(
  v: EditorValues,
  name: string,
  quickAdd: boolean,
  projects: Project[],
  sections: Section[],
  today: string,
): EditorValues {
  ready();
  return core.toggleLabel(v, name, quickAdd, projects, sections, today);
}

/** Turns the highlight ending at `caret` (UTF-16) into plain text; `null` when none ends there. */
export function unhighlightAt(
  v: EditorValues,
  projects: Project[],
  sections: Section[],
  today: string,
  caret: number,
): EditorValues | null {
  ready();
  return core.unhighlightAt(v, projects, sections, today, caret);
}

/** The values with every phrase the title says marked plain (a stored title's words stay text). */
export function withStoredPlain(
  v: EditorValues,
  projects: Project[],
  sections: Section[],
  today: string,
): EditorValues {
  ready();
  return core.withStoredPlain(v, projects, sections, today);
}

/** An edited title applied for saving; `null` when it has no name besides its phrases. */
export function applyTitle(
  v: EditorValues,
  existing: Recurrence | null,
  projects: Project[],
  sections: Section[],
  today: string,
): { values: EditorValues; rule: Recurrence | null } | null {
  ready();
  return core.applyTitle(v, existing, projects, sections, today);
}

/** The location menu's value (`project` or `project|section`) as a pick. */
export function locationPick(value: string): EditorPick {
  ready();
  return core.locationPick(value);
}

/** Each project, then its sections (nested): the project / section menu's items. */
export function locationChoices(projects: Project[], sections: Section[]): LocationChoice[] {
  ready();
  return core.locationChoices(projects, sections);
}

/** "Project" or "Project / Section"; "Inbox" for an unknown project. */
export function locationText(
  projectId: Id,
  sectionId: Id | null,
  projects: Project[],
  sections: Section[],
): string {
  ready();
  return core.locationText(projectId, sectionId, projects, sections);
}

/** "Moved from Wed 21 Oct" while a repeating task's date stands in for another occurrence, else null. */
export function movedNote(task: Task): string | null {
  ready();
  return core.movedNote(task) ?? null;
}

/** Whether saving `after` moves one occurrence of a repeating task (a streak can't carry over it). */
export function movesOccurrence(
  before: EditorValues,
  after: EditorValues,
  existing: Recurrence | null,
): boolean {
  ready();
  return core.movesOccurrence(before, after, existing);
}

/** What a typed repeat reads as ("every 3 tuesdays"), anchored on `date`; null when it isn't one. */
export function readRepeat(text: string, date: string, today: string): RepeatReading | null {
  ready();
  return core.readRepeat(text, date, today);
}

/** A repeat rule as text: "Every Mon, Wed". */
export function repeatText(rule: Recurrence): string {
  ready();
  return core.repeatText(rule);
}

/** A stored reminder as the editor edits it. */
export function reminderDraftOf(r: Reminder): ReminderDraft {
  ready();
  return core.reminderDraftOf(r);
}

/** What adding the editor's task sends: new labels, the task, then its reminders. */
export function addPlan(
  v: EditorValues,
  projects: Project[],
  sections: Section[],
  labels: Label[],
  today: string,
): AddPlan {
  ready();
  return core.addPlan(v, projects, sections, labels, today);
}

/** The new `#project` / `@label` names the save prompt lists. */
export function newNames(
  v: EditorValues,
  projects: Project[],
  sections: Section[],
  labels: Label[],
  today: string,
): NewNames {
  ready();
  return core.newNames(v, projects, sections, labels, today);
}

/** The values with those typed names left as plain text in the title. */
export function keepAsText(
  v: EditorValues,
  names: string[],
  projects: Project[],
  sections: Section[],
  today: string,
): EditorValues {
  ready();
  return core.keepAsText(v, names, projects, sections, today);
}

/** The values moved to a just-created project (its section cleared). */
export function withProject(v: EditorValues, projectId: Id): EditorValues {
  ready();
  return core.withProject(v, projectId);
}

/** A task add moved to a just-created project (its section cleared). */
export function taskWithProject(task: Action, projectId: Id): Action {
  ready();
  return core.taskWithProject(task, projectId);
}

/** What saving an edit sends: the move, reminder changes, then the field changes. */
export function editPlan(
  id: Id,
  before: EditorValues,
  after: EditorValues,
  existing: Recurrence | null,
  today: string,
): EditPlan {
  ready();
  return core.editPlan(id, before, after, existing, today);
}

/** The reminder changes as actions (removals, then additions). */
export function reminderActions(taskId: Id, before: ReminderDraft[], after: ReminderDraft[]): Action[] {
  ready();
  return core.reminderActions(taskId, before, after);
}

/** A reminder draft's text: "30 min before", "Tuesday 9 AM". */
export function reminderDraftLabel(d: ReminderDraft, today: string): string {
  ready();
  return core.reminderDraftLabel(d, today);
}

/** The relative reminder choices: [minutes before, text]. */
export function reminderPresets(): Array<[number, string]> {
  ready();
  return core.reminderPresets();
}

/** A custom reminder in plain words as `YYYY-MM-DDTHH:MM`; `null` without a date and a time. */
export function customReminderAt(text: string, today: string): string | null {
  ready();
  return core.customReminderAt(text, today);
}

/** The date picker's shortcuts: [label, hint, `YYYY-MM-DD`]. */
export function quickDates(today: string): Array<[string, string, string]> {
  ready();
  return core.quickDates(today);
}

/** A month as the date picker draws it; `weekStart` is 1 (Monday) … 7 (Sunday). */
export function monthView(
  year: number,
  month: number,
  weekStart: number,
  today: string,
  selected: string | null,
  rule: Recurrence | null,
): MonthView {
  ready();
  return core.monthView(year, month, weekStart, today, selected, rule);
}

/** The month `delta` months away: [year, month]. */
export function shiftMonth(year: number, month: number, delta: number): [number, number] {
  ready();
  return core.shiftMonth(year, month, delta);
}

/** The years the calendar and its month / year wheel offer. */
export function yearChoices(today: string, shown: number): number[] {
  ready();
  return core.yearChoices(today, shown);
}

/** The week each month of the years starts in, January first; `weekStart` as in `monthView`. */
export function monthFirstWeeks(firstYear: number, lastYear: number, weekStart: number): number[] {
  ready();
  return core.monthFirstWeeks(firstYear, lastYear, weekStart);
}

/** `count` weeks of the calendar's run of weeks, from week `from` as `monthFirstWeeks` counts them. */
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
  ready();
  return core.weekRows(firstYear, weekStart, from, count, shown[0], shown[1], today, selected, rule);
}

/** "January" … "December". */
export function monthNames(): string[] {
  ready();
  return core.monthNames();
}

/** A typed time ("5pm", "17:30") as `HH:MM`; `null` when it isn't one. */
export function typedTime(text: string): string | null {
  ready();
  return core.typedTime(text);
}

/** An `<input type="time">` value as `HH:MM`; `null` when cleared, `undefined` when unreadable. */
export function nativeTime(value: string): string | null | undefined {
  ready();
  return core.nativeTime(value);
}

// Settings (keel_core::settings).

/** The reminder menus: each one's choices, button text and checked value. */
export function reminderMenus(s: ReminderSettings): ReminderMenus {
  ready();
  return core.reminderMenus(s);
}

/** The settings after picking `value` in one menu. */
export function pickSetting(s: ReminderSettings, kind: MenuKind, value: string): ReminderSettings {
  ready();
  return core.pickSetting(s, kind, value);
}

/** When a planned reminder fires: "Today 5 PM", "Tue 9 AM". */
export function upcomingWhen(at: string, today: string): string {
  ready();
  return core.upcomingWhen(at, today);
}

/** One imported file's result: "Travel: 12 tasks, 2 sections". */
export function importLine(s: ImportSummary): string {
  ready();
  return core.importLine(s);
}
