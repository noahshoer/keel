import { fireEvent, render } from "@testing-library/react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import type { Task } from "../bindings";
import { makeTask } from "../ipc/fixtures";
import { invoke } from "../ipc/invoke";
import { loadCoreForTests } from "../test/wasm";
import { TaskNavProvider } from "./nav";
import { SubtaskRow } from "./SubtaskRow";

vi.mock("../ipc/invoke", async (real) => ({
  ...(await real<typeof import("../ipc/invoke")>()),
  invoke: vi.fn(() => Promise.resolve({ id_mapping: {} })),
}));

beforeAll(() => {
  loadCoreForTests();
});

afterEach(() => {
  vi.mocked(invoke).mockClear();
});

const task = (over: Partial<Task> = {}) =>
  makeTask({ id: "s", project_id: "p", content: "Push ups", ...over });
const show = (t: Task, done: boolean) =>
  render(
    <TaskNavProvider all={[t]}>
      <SubtaskRow row={{ task: t, progress: null, done }} onOpen={() => {}} onChanged={() => {}} />
    </TaskNavProvider>,
  ).container;

it("a repeating sub-task done this round shows done, with nothing for its circle to undo", () => {
  const c = show(task(), true);
  expect(c.querySelector(".tv-sub.done")).not.toBeNull();
  const circle = c.querySelector("button.check.done") as HTMLButtonElement;
  expect(circle.disabled).toBe(true);
  fireEvent.click(circle);
  expect(invoke).not.toHaveBeenCalled();
});

it("a checked sub-task's circle reopens it", () => {
  const c = show(task({ checked: true }), true);
  const circle = c.querySelector("button.check.done") as HTMLButtonElement;
  expect(circle.title).toBe("Mark as not done");
  fireEvent.click(circle);
  expect(invoke).toHaveBeenCalledOnce();
});

it("an open sub-task's circle completes it", () => {
  const c = show(task(), false);
  const circle = c.querySelector("button.check") as HTMLButtonElement;
  expect(circle.title).toBe("Complete sub-task");
  expect(circle.disabled).toBe(false);
});
