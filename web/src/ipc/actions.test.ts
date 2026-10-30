import { expect, it } from "vitest";
import {
  addFilter,
  addLabel,
  addProject,
  addSection,
  command,
  completeTask,
  deleteProject,
  deleteSection,
  deleteTask,
  renameLabel,
  renameProject,
  renameSection,
  setStreak,
  uncompleteTask,
  updateFilter,
} from "./actions";
import { invoke } from "./invoke";

it("builds commands with a uuid, and a temp id only for creates", () => {
  const add = command({ type: "project_add", name: "Garden" }, true);
  expect(add.uuid).toMatch(/^[0-9a-f-]{36}$/);
  expect(add.temp_id).toMatch(/^tmp-[0-9a-f-]{36}$/);
  const done = command({ type: "task_complete", id: "t" }, false);
  expect(done.temp_id).toBeNull();
  expect(done.uuid).not.toBe(add.uuid);
});

it("completing moves a task to the completed list; reopening brings it back", async () => {
  await completeTask("t-laundry");
  expect((await invoke("today_view", {})).map((t) => t.id)).not.toContain("t-laundry");
  expect((await invoke("completed_today", {})).map((t) => t.id)).toContain("t-laundry");
  // all_tasks keeps checked tasks (sub-task counts need them).
  expect((await invoke("all_tasks", {})).find((t) => t.id === "t-laundry")?.checked).toBe(true);
  await uncompleteTask("t-laundry");
  expect((await invoke("today_view", {})).map((t) => t.id)).toContain("t-laundry");
});

it("deleting a task deletes its sub-tasks", async () => {
  await deleteTask("t-laundry");
  const ids = (await invoke("all_tasks", {})).map((t) => t.id);
  expect(ids).not.toContain("t-laundry");
  expect(ids).not.toContain("t-sheets");
});

it("an unknown id is rejected with the backend's message", async () => {
  await expect(deleteTask("nope")).rejects.toBe("not found: nope");
});

it("each test starts from fresh fixtures", async () => {
  // The deletes above didn't leak into this test.
  expect((await invoke("all_tasks", {})).map((t) => t.id)).toContain("t-laundry");
});

it("streaks, projects, labels, sections and filters change through dispatch", async () => {
  await setStreak("t-laundry", true, 5);
  const t = (await invoke("all_tasks", {})).find((x) => x.id === "t-laundry");
  expect([t?.streak_enabled, t?.streak_goal]).toEqual([true, 5]);

  const res = await addProject("Garden");
  const pid = Object.values(res.id_mapping)[0] ?? "";
  await renameProject(pid, "Yard");
  expect((await invoke("list_projects", {})).map((p) => p.name)).toContain("Yard");
  await addSection(pid, "Beds");
  const [section] = await invoke("list_sections", { project_id: pid });
  await renameSection(section?.id ?? "", "Raised beds");
  expect((await invoke("list_sections", { project_id: pid })).map((s) => s.name)).toEqual(["Raised beds"]);
  await deleteSection(section?.id ?? "");
  expect(await invoke("list_sections", { project_id: pid })).toEqual([]);
  await deleteProject(pid);
  expect((await invoke("list_projects", {})).map((p) => p.name)).not.toContain("Yard");

  await addLabel("garden");
  const label = (await invoke("list_labels", {})).find((l) => l.name === "garden");
  await renameLabel(label?.id ?? "", "yard");
  expect((await invoke("list_labels", {})).map((l) => l.name)).toContain("yard");

  const f = await addFilter("Soon", "7 days");
  const fid = Object.values(f.id_mapping)[0] ?? "";
  await updateFilter(fid, "Next week", "7 days & p1");
  expect((await invoke("list_filters", {})).find((x) => x.id === fid)).toMatchObject({
    name: "Next week",
    query: "7 days & p1",
  });
});

it("returned values are copies, not the mock's state", async () => {
  const [first] = await invoke("list_projects", {});
  if (first) first.name = "changed";
  expect((await invoke("list_projects", {}))[0]?.name).not.toBe("changed");
});
