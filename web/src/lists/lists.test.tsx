import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, expect, it, vi } from "vitest";
import type { CardRow, Reminder, Task } from "../bindings";
import { localToday } from "../clock";
import * as fx from "../ipc/fixtures";
import { invoke } from "../ipc/invoke";
import { loadCoreForTests } from "../test/wasm";
import { CardList } from "./CardList";
import { CompletedRow } from "./CompletedRow";
import { TaskRow } from "./TaskRow";

beforeAll(() => {
  loadCoreForTests();
});

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

const today = localToday();
const fixture = (id: string): Task => {
  const t = fx.tasks.find((x) => x.id === id);
  if (!t) throw new Error(id);
  return structuredClone(t);
};
const row = (task: Task, progress: CardRow["progress"] = null): CardRow => ({ task, progress });
const taskEl = (container: HTMLElement) => container.querySelector(".task");
const circle = (container: HTMLElement) => {
  const b = container.querySelector<HTMLButtonElement>("button.check");
  if (!b) throw new Error("no circle");
  return b;
};
const tick = (ms: number) => act(async () => vi.advanceTimersByTimeAsync(ms));
const completed = async () => (await invoke("completed_today", {})).map((t) => t.id);

function renderRow(task: Task, opts: { reminders?: Reminder[]; progress?: CardRow["progress"] } = {}) {
  const onChanged = vi.fn();
  const view = render(
    <TaskRow
      row={row(task, opts.progress ?? null)}
      today={today}
      location="Home"
      reminders={opts.reminders ?? []}
      onChanged={onChanged}
    />,
  );
  return { ...view, onChanged };
}

it("completing fills the circle, switches the row off, then completes the task", async () => {
  const { container, onChanged } = renderRow(fixture("t-laundry"));
  fireEvent.click(circle(container));
  expect(taskEl(container)?.className).toBe("task completing");
  await tick(350);
  expect(taskEl(container)?.className).toBe("task completing switch-off");
  expect(await completed()).not.toContain("t-laundry");
  await tick(380);
  expect(await completed()).toContain("t-laundry");
  expect(onChanged).toHaveBeenCalledTimes(1);
  // Stays switched off until the reload drops it.
  expect(taskEl(container)?.className).toBe("task completing switch-off");
});

it("a second tap during the fill cancels", async () => {
  const { container, onChanged } = renderRow(fixture("t-laundry"));
  fireEvent.click(circle(container));
  await tick(100);
  fireEvent.click(circle(container));
  expect(taskEl(container)?.className).toBe("task");
  await tick(1000);
  expect(await completed()).not.toContain("t-laundry");
  expect(onChanged).not.toHaveBeenCalled();
});

it("a tap during the switch-off is ignored", async () => {
  const { container } = renderRow(fixture("t-laundry"));
  fireEvent.click(circle(container));
  await tick(400);
  fireEvent.click(circle(container));
  expect(taskEl(container)?.className).toBe("task completing switch-off");
  await tick(400);
  expect(await completed()).toContain("t-laundry");
});

it("a failed save brings the row back unchecked", async () => {
  const ghost = fx.makeTask({ id: "t-ghost", project_id: "p-home", content: "Not in the backend" });
  const { container, onChanged } = renderRow(ghost);
  fireEvent.click(circle(container));
  await tick(800);
  expect(taskEl(container)?.className).toBe("task");
  expect(onChanged).toHaveBeenCalledTimes(1);
});

it("a repeating task comes back on when its next date arrives", async () => {
  const task = fixture("t-laundry");
  const { container, rerender } = renderRow(task);
  fireEvent.click(circle(container));
  await tick(800);
  const next = structuredClone(task);
  if (next.due) next.due = { ...next.due, date: "2099-01-01", is_recurring: true };
  rerender(<TaskRow row={row(next)} today={today} location="Home" reminders={[]} onChanged={() => {}} />);
  expect(taskEl(container)?.className).toBe("task");
});

it("due chips are coloured by keel-core's bucket", () => {
  const { container } = render(
    <>
      <TaskRow
        row={row(fixture("t-taxes"))}
        today={today}
        location={null}
        reminders={[]}
        onChanged={() => {}}
      />
      <TaskRow
        row={row(fixture("t-laundry"))}
        today={today}
        location={null}
        reminders={[]}
        onChanged={() => {}}
      />
    </>,
  );
  const chips = [...container.querySelectorAll(".task-meta > span:first-child")].map((e) => e.className);
  expect(chips).toEqual(["due overdue", "due today"]);
});

it("a deadline due today is urgent; reminders show as a bell with their text", () => {
  const task = { ...fixture("t-laundry"), deadline: today };
  const { container } = renderRow(task, { reminders: fx.reminders });
  expect(container.querySelector(".deadline")?.className).toBe("deadline urgent");
  expect(container.querySelector(".deadline")?.textContent).toBe("Today");
  expect(container.querySelector(".reminder-tag")?.getAttribute("title")).toBe("30 min before");
  expect(container.querySelector(".label-tag")?.textContent).toBe("errand");
  expect(container.querySelector(".task-project")?.textContent).toBe("Home");
});

it("the green check shows only when every sub-task is done", () => {
  const all = renderRow(fixture("t-laundry"), { progress: { done: 2, total: 2 } });
  expect(all.container.querySelector(".done-check")).not.toBeNull();
  all.unmount();
  const some = renderRow(fixture("t-laundry"), { progress: { done: 1, total: 2 } });
  expect(some.container.querySelector(".task-count")?.textContent).toBe("1/2");
  expect(some.container.querySelector(".done-check")).toBeNull();
});

it("a repeating task's streak panel starts tracking", async () => {
  const task = fixture("t-laundry");
  if (task.due) task.due.is_recurring = true;
  const { onChanged } = renderRow(task);
  fireEvent.click(screen.getByTitle("Streak"));
  expect(screen.getByText("Track how many times in a row you complete this.")).toBeTruthy();
  fireEvent.click(screen.getByText("Track streak"));
  await tick(0);
  const saved = (await invoke("all_tasks", {})).find((t) => t.id === "t-laundry");
  expect(saved?.streak_enabled).toBe(true);
  expect(onChanged).toHaveBeenCalled();
});

it("a tracked streak shows current and best", () => {
  const task = { ...fixture("t-laundry"), streak_enabled: true, streak_current: 4, streak_best: 9 };
  if (task.due) task.due = { ...task.due, is_recurring: true };
  const { container } = renderRow(task);
  expect(container.querySelector(".streak")?.textContent).toBe("🔥 4");
  fireEvent.click(screen.getByTitle("Streak"));
  expect(container.querySelector(".streak-panel")?.textContent).toContain("Best 9");
  expect(screen.getByText("Stop tracking")).toBeTruthy();
});

it("the delete button deletes the task", async () => {
  const { onChanged } = renderRow(fixture("t-groceries"));
  fireEvent.click(screen.getByTitle("Delete task"));
  await tick(0);
  expect((await invoke("all_tasks", {})).map((t) => t.id)).not.toContain("t-groceries");
  expect(onChanged).toHaveBeenCalled();
});

it("a completed row reopens from its filled check", async () => {
  const onChanged = vi.fn();
  const { container } = render(
    <CompletedRow task={fixture("t-done")} today={today} location="Home" onChanged={onChanged} />,
  );
  expect(container.querySelector(".task")?.className).toBe("task completed");
  expect(container.querySelector(".done-when")?.textContent).toBe("Completed · Today");
  fireEvent.click(screen.getByTitle("Mark as not done"));
  await tick(0);
  expect((await invoke("all_tasks", {})).find((t) => t.id === "t-done")?.checked).toBe(false);
  expect(onChanged).toHaveBeenCalled();
});

it("a card whose head doesn't match shows it as context, with its sub-task rows", () => {
  const parent = fixture("t-laundry");
  const sub = fixture("t-sheets");
  const { container } = render(
    <CardList
      cards={[{ head: row(parent, { done: 0, total: 1 }), head_matches: false, rows: [row(sub)] }]}
      today={today}
      locations={null}
      reminders={[]}
      onChanged={() => {}}
    />,
  );
  const head = container.querySelector(".task-card > .card-head");
  expect(head?.querySelector(".pname")?.textContent).toBe("Do laundry");
  expect(head?.querySelector(".card-flag")?.className).toBe("card-flag p1");
  expect(head?.querySelector(".task-count")?.textContent).toBe("0/1");
  expect([...container.querySelectorAll(".task-row .task-content")].map((e) => e.textContent)).toEqual([
    "Wash the sheets",
  ]);
  expect(container.querySelector(".task-project")).toBeNull();
});

it("the streak goal saves once, when the field is left", async () => {
  const task = { ...fixture("t-laundry"), streak_enabled: true };
  if (task.due) task.due = { ...task.due, is_recurring: true };
  const { container } = renderRow(task);
  fireEvent.click(screen.getByTitle("Streak"));
  const goal = container.querySelector<HTMLInputElement>(".streak-panel input") as HTMLInputElement;
  fireEvent.change(goal, { target: { value: "1" } });
  fireEvent.change(goal, { target: { value: "12" } });
  await tick(0);
  expect((await invoke("all_tasks", {})).find((t) => t.id === "t-laundry")?.streak_goal).toBeNull();
  fireEvent.blur(goal);
  await tick(0);
  expect((await invoke("all_tasks", {})).find((t) => t.id === "t-laundry")?.streak_goal).toBe(12);
});

it("a completion stops when its row goes away", async () => {
  const { container, unmount, onChanged } = renderRow(fixture("t-laundry"));
  fireEvent.click(circle(container));
  await tick(100);
  unmount();
  await tick(1000);
  expect(await completed()).not.toContain("t-laundry");
  expect(onChanged).not.toHaveBeenCalled();
});
