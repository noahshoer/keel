import { fireEvent, render } from "@testing-library/react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import type { Task } from "../bindings";
import { makeTask } from "../ipc/fixtures";
import { invoke } from "../ipc/invoke";
import { loadCoreForTests } from "../test/wasm";
import { TaskNavProvider } from "./nav";
import { type TaskNav, TaskNavContext } from "./nav-context";
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

const sent = () =>
  (vi.mocked(invoke).mock.lastCall as unknown as [string, { cmd: { action: unknown } }])[1].cmd.action;

it("a repeating sub-task done this round shows done, and its circle undoes it", () => {
  const c = show(task(), true);
  expect(c.querySelector(".tv-sub.done")).not.toBeNull();
  const circle = c.querySelector("button.check.done") as HTMLButtonElement;
  expect(circle.title).toBe("Mark as not done");
  fireEvent.click(circle);
  expect(sent()).toEqual({ type: "task_uncomplete", id: "s" });
});

it("a checked sub-task's circle reopens it", () => {
  const c = show(task({ checked: true }), true);
  const circle = c.querySelector("button.check.done") as HTMLButtonElement;
  expect(circle.title).toBe("Mark as not done");
  fireEvent.click(circle);
  // Reopening plays no vibration.
  expect(invoke).toHaveBeenCalledOnce();
});

it("an open sub-task's circle completes it", () => {
  const c = show(task(), false);
  const circle = c.querySelector("button.check") as HTMLButtonElement;
  expect(circle.title).toBe("Complete sub-task");
  fireEvent.click(circle);
  expect(sent()).toEqual({ type: "task_complete", id: "s" });
  expect(invoke).toHaveBeenCalledWith("haptic", { kind: "complete" });
});

it("a sub-task with a streak shows its count after its name", () => {
  const c = show(task({ streak_enabled: true, streak_current: 6 }), false);
  const badge = c.querySelector(".tv-sub .streak");
  expect(badge?.textContent).toBe("6");
  expect(badge?.previousElementSibling?.classList.contains("task-content")).toBe(true);
});

it("a sub-task without a streak shows no count", () => {
  expect(show(task(), false).querySelector(".streak")).toBeNull();
});

const stubNav = (completedHere: () => void): TaskNav => ({
  stack: ["p"],
  showCompleted: false,
  addSub: false,
  closeAsked: false,
  open: () => {},
  openAdding: () => {},
  push: () => {},
  back: () => {},
  close: () => {},
  askClose: () => {},
  setShowCompleted: () => {},
  clearAddSub: () => {},
  clearCloseAsked: () => {},
  completedHere,
  holdCircles: () => true,
  releaseCircles: () => Promise.resolve(),
  prune: () => {},
});

async function completeWith(autoCompleted: string[]) {
  vi.mocked(invoke).mockImplementation(() =>
    Promise.resolve({ id_mapping: {}, auto_completed: autoCompleted }),
  );
  const completedHere = vi.fn();
  const onChanged = vi.fn();
  const t = task({ parent_id: "p" });
  const c = render(
    <TaskNavContext.Provider value={stubNav(completedHere)}>
      <SubtaskRow row={{ task: t, progress: null, done: false }} onOpen={() => {}} onChanged={onChanged} />
    </TaskNavContext.Provider>,
  ).container;
  fireEvent.click(c.querySelector("button.check") as HTMLButtonElement);
  await vi.waitFor(() => expect(onChanged).toHaveBeenCalled());
  return completedHere;
}

it("completing the last sub-task closes the parent's view when the parent completed with it", async () => {
  expect(await completeWith(["p"])).toHaveBeenCalledOnce();
});

it("completing a sub-task leaves the view open when the parent didn't complete", async () => {
  expect(await completeWith([])).not.toHaveBeenCalled();
});
