// The editor's exports (keel_core::editor through crates/core-wasm), with
// 2026-10-30 (a Friday) as today.
import { beforeAll, describe, expect, it } from "vitest";
import * as fx from "../ipc/fixtures";
import { loadCoreForTests } from "../test/wasm";
import {
  addPlan,
  applyPick,
  blankValues,
  customReminderAt,
  editorView,
  editPlan,
  locationText,
  monthNames,
  monthView,
  nativeTime,
  quickDates,
  reminderDraftOf,
  repeatText,
  shiftMonth,
  typedTime,
  unhighlightAt,
  withTitle,
  yearChoices,
} from ".";

const TODAY = "2026-10-30";

beforeAll(() => {
  loadCoreForTests();
});

const typed = (text: string) => withTitle(blankValues("p-inbox", null, "", []), text);
const kinds = (text: string) =>
  editorView(typed(text), true, null, null, [], [], TODAY).segments.map((s) => s.kind);

describe("editor over wasm", () => {
  it("highlights the title and can submit", () => {
    const view = editorView(typed("Call mom tomorrow p1"), true, null, null, [], [], TODAY);
    expect(view.segments.map((s) => s.kind)).toEqual([null, "Date", null, "Priority"]);
    expect(view.segments[3]?.level).toBe(1);
    expect(view.can_submit).toBe(true);
    expect(view.preview.map((c) => c.text)).toEqual(["Tomorrow", "P1"]);
  });

  it("a date pick strips the typed date", () => {
    const v = applyPick(
      typed("call mom tomorrow"),
      { type: "date", date: "2026-11-06", time: "" },
      true,
      [],
      [],
      TODAY,
    );
    expect(v.content).toBe("call mom");
    expect(v.date).toBe("2026-11-06");
  });

  it("un-highlights at the caret", () => {
    const v = typed("Call mom tomorrow p1");
    expect(unhighlightAt(v, [], [], TODAY, 0)).toBeNull();
    const after = unhighlightAt(v, [], [], TODAY, "Call mom tomorrow".length);
    expect(after).not.toBeNull();
    if (after === null) return;
    expect(editorView(after, true, null, null, [], [], TODAY).segments.map((s) => s.kind)).not.toContain(
      "Date",
    );
    expect(kinds("Café tomorrow")).toContain("Date");
  });

  it("plans adds and edits", () => {
    expect(addPlan(typed("x @new"), [], [], [], TODAY)).toMatchObject({
      labels: ["new"],
      task: { type: "task_add" },
    });
    const before = typed("x");
    const plan = editPlan("t1", before, { ...before, priority: "1" }, null, TODAY);
    expect(plan.moved).toBeNull();
    expect(plan.update).toMatchObject({ type: "task_update", priority: "P1" });
  });

  it("reminders, dates and times", () => {
    expect(customReminderAt("tomorrow 9am", TODAY)).toBe("2026-10-31T09:00");
    expect(customReminderAt("tomorrow", TODAY)).toBeNull();
    expect(quickDates(TODAY)[0]).toEqual(["Today", "Fri", "2026-10-30"]);
    expect(typedTime("5pm")).toBe("17:00");
    expect(typedTime("bad")).toBeNull();
    expect(nativeTime("")).toBeNull();
    expect(nativeTime("07:05:00")).toBe("07:05");
    expect(nativeTime("5pm")).toBeUndefined();
  });

  it("calendar", () => {
    const view = monthView(2026, 10, 1, TODAY, null, null);
    expect(view.title).toBe("October 2026");
    expect(view.weeks).toHaveLength(6);
    expect(view.weeks[0]?.[0]?.date).toBe("2026-09-28");
    expect(shiftMonth(2026, 12, 1)).toEqual([2027, 1]);
    expect(yearChoices(TODAY, 2026)[0]).toBe(2025);
    expect(monthNames()[0]).toBe("January");
  });

  it("the task view's texts: a rule, a location and a stored reminder", () => {
    expect(repeatText({ Weekly: [0, 2] })).toBe("Every Mon, Wed");
    expect(locationText("p-home", null, fx.projects, fx.sections)).toBe("Home");
    expect(locationText("p-home", "s-chores", fx.projects, fx.sections)).toBe("Home / Chores");
    expect(locationText("gone", null, fx.projects, fx.sections)).toBe("Inbox");
    const relative = fx.reminders[0];
    if (relative === undefined) throw new Error("no reminder fixture");
    expect(reminderDraftOf(relative)).toEqual({ id: "r-laundry", minuteOffset: 30, at: null });
    expect(
      reminderDraftOf({
        ...relative,
        kind: "Absolute",
        minute_offset: null,
        due: { date: "2026-10-31T09:00:00", timezone: null, string: "", lang: "en", is_recurring: false },
      }),
    ).toEqual({ id: "r-laundry", minuteOffset: null, at: "2026-10-31T09:00" });
  });
});
