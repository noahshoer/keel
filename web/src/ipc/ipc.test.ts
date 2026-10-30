import { afterEach, expect, expectTypeOf, test, vi } from "vitest";
import type { ApplyResult, Task } from "../bindings";
import type { Commands } from "../bindings/Commands";
import commandNames from "../bindings/commands.json";
import { COMMAND_TYPES } from "./commands";
import { invoke } from "./invoke";

afterEach(() => {
  delete window.__TAURI__;
});

function stubTauri(impl: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>) {
  const spy = vi.fn(impl);
  window.__TAURI__ = { core: { invoke: spy } };
  return spy;
}

test("every command has a client entry", () => {
  expect(Object.keys(COMMAND_TYPES).sort()).toEqual(commandNames);
});

test("return types are generated from Rust", () => {
  expectTypeOf<Commands["today_view"]["ret"]>().toEqualTypeOf<Task[]>();
  expectTypeOf<Commands["dispatch"]["ret"]>().toEqualTypeOf<ApplyResult>();
  expectTypeOf<Commands["check_filter"]["ret"]>().toEqualTypeOf<null>();
  expectTypeOf<Commands["reminders_when_closed"]["ret"]>().toEqualTypeOf<boolean>();
});

test("sends args wrapped and snake_case", async () => {
  const spy = stubTauri(async () => []);
  await invoke("list_tasks", { project_id: "p1" });
  expect(spy).toHaveBeenCalledWith("list_tasks", { args: { project_id: "p1" } });
});

test("uses the mock without Tauri", async () => {
  const tasks = await invoke("today_view", {});
  expect(tasks.map((t) => t.id)).toEqual(["t-laundry", "t-sheets"]);
});

test("the mock applies a TaskAdd and a TaskComplete", async () => {
  const before = await invoke("list_tasks", { project_id: "p-home" });
  const res = await invoke("dispatch", {
    cmd: {
      uuid: "u1",
      temp_id: "tmp1",
      action: { type: "task_add", content: "Water plants", project_id: "p-home" },
    },
  });
  const realId = res.id_mapping["tmp1"];
  expect(realId).toBeTruthy();
  const added = await invoke("list_tasks", { project_id: "p-home" });
  expect(added.length).toBe(before.length + 1);
  await invoke("dispatch", { cmd: { uuid: "u2", action: { type: "task_complete", id: realId ?? "" } } });
  const after = await invoke("list_tasks", { project_id: "p-home" });
  expect(after.length).toBe(before.length);
});

test("rejects with the backend message", async () => {
  stubTauri(() => Promise.reject("no such task"));
  await expect(invoke("list_tasks", { project_id: "x" })).rejects.toBe("no such task");
});

test("a production build without Tauri throws", async () => {
  vi.stubEnv("DEV", false);
  try {
    await expect(invoke("today_view", {})).rejects.toThrow("Keel must run inside its app");
  } finally {
    vi.unstubAllEnvs();
  }
});

test("mock views are static lists; only a task's own flags change them", async () => {
  const a = await invoke("search", { query: "laundry" });
  const b = await invoke("search", { query: "zzz" });
  expect(b).toEqual(a);
  await invoke("dispatch", { cmd: { uuid: "u3", action: { type: "task_complete", id: "t-sheets" } } });
  const today = await invoke("today_view", {});
  expect(today.map((t) => t.id)).toEqual(["t-laundry"]);
});
