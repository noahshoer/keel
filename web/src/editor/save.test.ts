// Saving the editor's values through the mock backend (2026-10-30 as today).
import { beforeAll, expect, it, vi } from "vitest";
import { blankValues, valuesFromTask, withTitle } from "../core";
import * as fx from "../ipc/fixtures";
import { invoke } from "../ipc/invoke";
import { mockHandlers } from "../ipc/mock";
import { loadCoreForTests } from "../test/wasm";
import { addFromEditor, saveEdit } from "./save";

const TODAY = "2026-10-30";

beforeAll(() => {
  loadCoreForTests();
});

const typed = (text: string) => withTitle(blankValues("p-inbox", null, "", []), text);
const task = async (content: string) => (await invoke("all_tasks", {})).find((t) => t.content === content);

it("adds the labels it mentions, then the task", async () => {
  await addFromEditor(typed("Buy milk tomorrow @Errands"), fx.projects, fx.sections, fx.labels, TODAY);
  expect((await invoke("list_labels", {})).map((l) => l.name)).toContain("Errands");
  const added = await task("Buy milk");
  expect(added?.due?.date).toBe("2026-10-31");
  expect(added?.labels).toEqual(["Errands"]);
});

it("puts picked reminders on the new task", async () => {
  const v = { ...typed("Pay rent tomorrow"), reminders: [{ id: null, minuteOffset: 60, at: null }] };
  await addFromEditor(v, fx.projects, fx.sections, fx.labels, TODAY);
  const id = (await task("Pay rent"))?.id;
  const mine = (await invoke("list_reminders", {})).filter((r) => r.task_id === id);
  expect(mine.map((r) => r.minute_offset)).toEqual([60]);
});

it("rejects when the task isn't added", async () => {
  const dispatch = mockHandlers.dispatch;
  const spy = vi
    .spyOn(mockHandlers, "dispatch")
    .mockImplementation((args) =>
      args.cmd.action.type === "task_add" ? Promise.reject("disk full") : dispatch(args),
    );
  try {
    await expect(addFromEditor(typed("Orphan"), fx.projects, fx.sections, fx.labels, TODAY)).rejects.toBe(
      "disk full",
    );
  } finally {
    spy.mockRestore();
  }
});

it("saves a move, a reminder change and a field change", async () => {
  const t = await task("Do laundry");
  if (t === undefined) throw new Error("fixture");
  const before = { ...valuesFromTask(t), reminders: [{ id: "r-laundry", minuteOffset: 30, at: null }] };
  const after = { ...before, section_id: null, priority: "3", reminders: [] };
  await saveEdit(t.id, before, after, null, TODAY);
  const saved = await task("Do laundry");
  expect(saved?.section_id).toBeNull();
  expect(saved?.priority).toBe("P3");
  expect((await invoke("list_reminders", {})).some((r) => r.id === "r-laundry")).toBe(false);
});
