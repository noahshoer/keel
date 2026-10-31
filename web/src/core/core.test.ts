import { beforeAll, describe, expect, it } from "vitest";
import type { Task } from "../bindings";
import { makeTask } from "../ipc/fixtures";
import { loadCoreForTests } from "../test/wasm";
import {
  allDone,
  cards,
  chain,
  dateGroups,
  daySubtitle,
  deadlineUrgent,
  dueBucket,
  dueDateOf,
  dueLabel,
  dueTime,
  fieldLayout,
  importLine,
  pickSetting,
  progress,
  progressLong,
  progressShort,
  reminderLabel,
  reminderMenus,
  taskDetail,
  timeLabel,
  upcomingTasks,
  upcomingWhen,
  valuesFromTask,
} from ".";

beforeAll(() => {
  loadCoreForTests();
});

describe("keel-core over wasm", () => {
  it("due label matches Rust", () => {
    // Same cases as `due_label` in crates/core/src/views.rs.
    expect(dueLabel("2026-10-30", "2026-10-29")).toBe("Tomorrow");
    expect(dueLabel("2026-10-29", "2026-10-29")).toBe("Today");
    expect(dueLabel("2027-03-04", "2026-10-19")).toBe("4 Mar 2027");
  });

  it("due label rejects a malformed date", () => {
    expect(() => dueLabel("30/10/2026", "2026-10-29")).toThrow(/invalid date "30\/10\/2026"/);
    // Strict: a datetime goes through dueDateOf first.
    expect(() => dueLabel("2026-10-30T17:00:00", "2026-10-29")).toThrow(/invalid date/);
  });

  it("due date and time of a task match Rust", () => {
    // Same cases as due_date_of_reads_date_and_datetime / reads_time_from_due in views.rs.
    const due = (date: string): Task["due"] => ({
      date,
      timezone: null,
      string: "",
      lang: "en",
      is_recurring: false,
    });
    const none = makeTask({ id: "t", project_id: "p", content: "x" });
    expect(dueDateOf(none)).toBeNull();
    expect(dueTime(none)).toBeNull();
    const dateOnly = makeTask({ id: "t", project_id: "p", content: "x", due: due("2026-10-19") });
    expect(dueDateOf(dateOnly)).toBe("2026-10-19");
    expect(dueTime(dateOnly)).toBeNull();
    const timed = makeTask({ id: "t", project_id: "p", content: "x", due: due("2026-10-19T17:30:00") });
    expect(dueDateOf(timed)).toBe("2026-10-19");
    expect(dueTime(timed)).toBe("17:30:00");
    expect(timeLabel("17:30:00")).toBe("5:30 PM");
  });

  it("time label matches Rust", () => {
    expect(timeLabel("17:00")).toBe("5 PM");
    expect(timeLabel("09:05")).toBe("9:05 AM");
    expect(timeLabel("17:30:00")).toBe("5:30 PM");
  });

  it("cards hide sub-tasks under their parent", () => {
    const parent = makeTask({ id: "t-parent", project_id: "p-home", content: "Pack" });
    const child = makeTask({ id: "t-child", project_id: "p-home", content: "Socks", parent_id: "t-parent" });
    const all = [parent, child];
    const out = cards(all, all, "TopLevel");
    expect(out).toHaveLength(1);
    expect(out[0]?.head.task.id).toBe("t-parent");
    expect(out[0]?.head.progress).toEqual({ done: 0, total: 1 });
    expect(out[0]?.rows).toEqual([]);
    // Round-trips the task unchanged (wire shape equals the binding).
    expect(out[0]?.head.task).toEqual(parent);
  });

  it("progress counts direct sub-tasks", () => {
    const parent = makeTask({ id: "a", project_id: "p", content: "A" });
    const done = makeTask({ id: "b", project_id: "p", content: "B", parent_id: "a", checked: true });
    const open = makeTask({ id: "c", project_id: "p", content: "C", parent_id: "a" });
    expect(progress("a", [parent, done, open])).toEqual({ done: 1, total: 2 });
    expect(progress("b", [parent, done, open])).toBeNull();
  });

  // The cases below are the Rust tests' (views.rs date_group_tests,
  // reminders.rs label_tests), with 2026-10-30 (a Friday) as today.
  const dated = (id: string, date: string): Task =>
    makeTask({
      id,
      project_id: "p",
      content: id,
      due: { date, timezone: null, string: "", lang: "en", is_recurring: false },
    });

  it("due bucket matches Rust", () => {
    expect(dueBucket("2026-10-29", "2026-10-30")).toBe("Overdue");
    expect(dueBucket("2026-10-30", "2026-10-30")).toBe("Today");
    expect(dueBucket("2026-10-31", "2026-10-30")).toBe("Tomorrow");
    expect(dueBucket("2026-11-02", "2026-10-30")).toBe("ThisWeek");
    expect(dueBucket("2026-11-20", "2026-10-30")).toBe("Later");
  });

  it("all done needs at least one sub-task, all checked", () => {
    expect(allDone({ done: 2, total: 2 })).toBe(true);
    expect(allDone({ done: 1, total: 2 })).toBe(false);
    expect(allDone({ done: 0, total: 0 })).toBe(false);
  });

  it("today splits overdue only when there is some", () => {
    expect(dateGroups("Today", [dated("Laundry", "2026-10-30")], "2026-10-30").map((g) => g.title)).toEqual([
      null,
    ]);
    const groups = dateGroups(
      "Today",
      [dated("Laundry", "2026-10-30"), dated("Taxes", "2026-10-28")],
      "2026-10-30",
    );
    expect(groups.map((g) => [g.title, g.muted])).toEqual([
      ["Overdue", ""],
      ["Today", "30 Oct · Friday"],
    ]);
    expect(groups[0]?.tasks.map((t) => t.id)).toEqual(["Taxes"]);
  });

  it("upcoming labels today and tomorrow", () => {
    const groups = dateGroups(
      "Upcoming",
      [dated("A", "2026-10-30"), dated("B", "2026-10-31"), dated("C", "2026-11-01")],
      "2026-10-30",
    );
    expect(groups.map((g) => [g.title, g.muted])).toEqual([
      ["30 Oct", "Today · Friday"],
      ["31 Oct", "Tomorrow · Saturday"],
      ["1 Nov", "Sunday"],
    ]);
  });

  it("upcoming tasks put overdue first and drop today's from the Today query", () => {
    const now = dated("Now", "2026-10-30");
    const out = upcomingTasks(
      [now, dated("Late", "2026-10-29")],
      [now, dated("Soon", "2026-11-02")],
      "2026-10-30",
    );
    expect(out.map((t) => t.id)).toEqual(["Late", "Now", "Soon"]);
  });

  it("a deadline is urgent on the day and after", () => {
    expect(deadlineUrgent("2026-10-30", "2026-10-30")).toBe(true);
    expect(deadlineUrgent("2026-10-29", "2026-10-30")).toBe(true);
    expect(deadlineUrgent("2026-10-31", "2026-10-30")).toBe(false);
  });

  it("reminder labels match Rust", () => {
    const base = { id: "r", task_id: "t", is_deleted: false };
    expect(reminderLabel({ ...base, kind: "Relative", due: null, minute_offset: 30 }, "2026-10-30")).toBe(
      "30 min before",
    );
    const at = "2026-11-03T09:00:00";
    const due = { date: at, timezone: null, string: at, lang: "en", is_recurring: false };
    expect(reminderLabel({ ...base, kind: "Absolute", due, minute_offset: null }, "2026-10-30")).toBe(
      "Tuesday 9 AM",
    );
  });

  it("the day subtitle and the sub-task count come from Rust", () => {
    expect(daySubtitle("2026-10-30")).toBe("Friday 30 Oct");
    expect(daySubtitle("2026-11-01")).toBe("Sunday 1 Nov");
    expect(progressShort({ done: 2, total: 5 })).toBe("2/5");
  });

  it("field layout splits set fields from pills", () => {
    const t = makeTask({
      id: "t",
      project_id: "p-inbox",
      content: "A",
      description: "Notes",
      priority: "P2",
      labels: ["errand"],
      due: {
        date: "2026-10-30",
        string: "today",
        is_recurring: false,
        recurrence: null,
        lang: "en",
        timezone: null,
      },
    });
    const out = fieldLayout(valuesFromTask(t), "p-inbox");
    expect(out.fields).toEqual(["Description", "Date", "Priority", "Labels"]);
    expect(out.missing.map((m) => m.label)).toEqual(["+ Repeat", "+ Deadline", "+ Reminders", "+ Location"]);
  });

  it("task detail hides checked sub-tasks unless asked", () => {
    const parent = makeTask({ id: "p", project_id: "p-home", content: "Pack" });
    const a = makeTask({ id: "a", project_id: "p-home", content: "A", parent_id: "p" });
    const b = makeTask({
      id: "b",
      project_id: "p-home",
      content: "B",
      parent_id: "p",
      checked: true,
      child_order: 1,
    });
    const all = [parent, a, b];
    const hidden = taskDetail("a", all, false);
    expect(hidden?.parent?.id).toBe("p");
    expect(taskDetail("p", all, false)?.visible).toHaveLength(1);
    expect(taskDetail("p", all, true)?.visible).toHaveLength(2);
    expect(taskDetail("nope", all, false)).toBeNull();
  });

  it("chain runs from the top parent down", () => {
    const p = makeTask({ id: "P", project_id: "x", content: "P" });
    const c = makeTask({ id: "C", project_id: "x", content: "C", parent_id: "P" });
    const g = makeTask({ id: "G", project_id: "x", content: "G", parent_id: "C" });
    expect(chain("G", [p, c, g])).toEqual(["P", "C", "G"]);
  });

  it("progress long reads 2 of 5", () => {
    expect(progressLong({ done: 2, total: 5 })).toBe("2 of 5");
  });
});

describe("settings", () => {
  const settings = { enabled: true, offset_minutes: 0, all_day_time: "09:00:00", daily_summary: null };

  it("reminder menus select a stored time with seconds", () => {
    const m = reminderMenus(settings);
    expect(m.all_day.selected).toBe("09:00");
    expect(m.all_day.text).toBe("9 AM");
    expect(m.summary.selected).toBe("off");
    expect(m.offset.items.map((i) => i.label)[1]).toBe("5 minutes before");
  });

  it("a pick changes one setting", () => {
    const next = pickSetting(settings, "Summary", "07:30");
    expect(next.daily_summary).toBe("07:30:00");
    expect(next.all_day_time).toBe("09:00:00");
    expect(pickSetting(settings, "Offset", "15").offset_minutes).toBe(15);
  });

  it("upcoming times read from today", () => {
    expect(upcomingWhen("2026-10-30T17:00:00", "2026-10-30")).toBe("Today 5 PM");
    expect(upcomingWhen("2026-10-31T09:30:00", "2026-10-30")).toBe("Tomorrow 9:30 AM");
  });

  it("an import line counts what was created", () => {
    const line = importLine({
      project_id: "p",
      project_name: "Travel",
      tasks: 2,
      sections: 1,
      labels_created: 0,
      comments: 0,
      kept_as_text: 0,
    });
    expect(line).toBe("Travel: 2 tasks, 1 section");
  });
});
