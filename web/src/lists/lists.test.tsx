import { act, fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeAll, beforeEach, expect, it, vi } from "vitest";
import type { CardRow, Label, Reminder, Task } from "../bindings";
import { localToday } from "../clock";
import { ROW_PRESS } from "../gestures/long-press";
import * as fx from "../ipc/fixtures";
import { invoke } from "../ipc/invoke";
import { TaskNavProvider } from "../taskview/nav";
import { useTaskNav } from "../taskview/nav-context";
import { loadCoreForTests } from "../test/wasm";
import { CardList } from "./CardList";
import { CompletedRow } from "./CompletedRow";
import { LocationsContext } from "./locations";
import type { RowLocation } from "./rowLayout";
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
/** The task view's nav, shown as "stack|add" so a test can read what a tap opened. */
// biome-ignore lint/style/useComponentExportOnlyModules: a test probe, never hot-reloaded.
function NavProbe() {
  const nav = useTaskNav();
  return <output data-testid="nav">{`${nav.stack.join(",")}|${nav.addSub ? "add" : ""}`}</output>;
}
const wrapper = ({ children }: { children: ReactNode }) => (
  <LocationsContext.Provider value={{ projects: fx.projects, sections: fx.sections }}>
    <TaskNavProvider all={fx.tasks}>
      {children}
      <NavProbe />
    </TaskNavProvider>
  </LocationsContext.Provider>
);
const opened = () => screen.getByTestId("nav").textContent;
const row = (task: Task, progress: CardRow["progress"] = null): CardRow => ({ task, progress });
const taskEl = (container: HTMLElement) => container.querySelector(".task");
const circle = (container: HTMLElement) => {
  const b = container.querySelector<HTMLButtonElement>("button.check");
  if (!b) throw new Error("no circle");
  return b;
};
const tick = (ms: number) => act(async () => vi.advanceTimersByTimeAsync(ms));
const completed = async () => (await invoke("completed_today", {})).map((t) => t.id);

function renderRow(
  task: Task,
  opts: {
    reminders?: Reminder[];
    progress?: CardRow["progress"];
    location?: RowLocation;
    labels?: Label[];
  } = {},
) {
  const onChanged = vi.fn();
  const view = render(
    <TaskRow
      row={row(task, opts.progress ?? null)}
      today={today}
      location={opts.location ?? { project: "Home", section: null }}
      labels={opts.labels ?? fx.labels}
      reminders={opts.reminders ?? []}
      onChanged={onChanged}
    />,
    { wrapper },
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
  rerender(
    <TaskRow
      row={row(next)}
      today={today}
      location={{ project: "Home", section: null }}
      labels={fx.labels}
      reminders={[]}
      onChanged={() => {}}
    />,
  );
  expect(taskEl(container)?.className).toBe("task");
});

it("due chips are coloured by keel-core's bucket", () => {
  const { container } = render(
    <>
      <TaskRow
        row={row(fixture("t-taxes"))}
        today={today}
        location={{ project: "Inbox", section: null }}
        labels={[]}
        reminders={[]}
        onChanged={() => {}}
      />
      <TaskRow
        row={row(fixture("t-laundry"))}
        today={today}
        location={{ project: "Home", section: null }}
        labels={[]}
        reminders={[]}
        onChanged={() => {}}
      />
    </>,
    { wrapper },
  );
  const chips = [...container.querySelectorAll(".task-when .due")].map((e) => e.className);
  expect(container.querySelector(".task-when .due svg")).not.toBeNull();
  expect(chips).toEqual(["due overdue", "due today"]);
});

const label = (name: string, color: string): Label => ({
  id: `l-${name}`,
  name,
  color,
  item_order: 0,
  is_favorite: false,
  is_deleted: false,
});
const childClasses = (el: Element | null) => [...(el?.children ?? [])].map((c) => c.className.split(" ")[0]);

it("rows run title and streak, deadline, description, then the bottom rows", () => {
  const task = {
    ...fixture("t-laundry"),
    deadline: today,
    description: "All of it",
    streak_enabled: true,
    streak_current: 4,
  };
  const { container } = renderRow(task, { progress: { done: 1, total: 2 } });
  expect(childClasses(container.querySelector(".task-body"))).toEqual([
    "task-title",
    "task-deadline",
    "task-desc",
    "task-when",
    "task-bottom",
  ]);
  expect(childClasses(container.querySelector(".task-title"))).toEqual(["task-content", "streak"]);
  expect(container.querySelector(".streak")?.textContent).toBe("4");
  expect(container.querySelector(".streak svg")).not.toBeNull();
  expect(container.querySelector(".task-deadline .deadline")?.textContent).toBe("Today");
  expect(container.querySelector(".task-deadline svg")).not.toBeNull();
  expect(container.querySelector(".task-desc")?.textContent).toBe("All of it");
});

it("a row with no fields set has only the title and the project", () => {
  const task = { ...fixture("t-taxes"), due: null, labels: [], deadline: null, description: "" };
  const { container } = renderRow(task);
  expect(childClasses(container.querySelector(".task-body"))).toEqual(["task-title", "task-bottom"]);
  expect(childClasses(container.querySelector(".task-bottom"))).toEqual(["task-project"]);
});

it("the when group runs sub-tasks, date, repeat icon, bell", () => {
  const base = fixture("t-laundry");
  const task = { ...base, due: base.due && { ...base.due, is_recurring: true } };
  const { container } = renderRow(task, { reminders: fx.reminders, progress: { done: 1, total: 2 } });
  const when = container.querySelector(".task-when");
  expect(childClasses(when)).toEqual(["task-subtasks", "due", "task-repeat", "reminder-tag"]);
  expect(when?.querySelector(".task-repeat")?.textContent).toBe("");
  expect(when?.querySelector(".reminder-tag")?.getAttribute("title")).toBe("30 min before");
});

it("the project shares a row with the when group when there are no labels", () => {
  const task = { ...fixture("t-laundry"), labels: [] };
  const { container } = renderRow(task);
  expect(childClasses(container.querySelector(".task-body"))).toEqual(["task-title", "task-bottom"]);
  expect(childClasses(container.querySelector(".task-bottom"))).toEqual(["task-when", "task-project"]);
});

it("labels share a row with the project when there is no when group", () => {
  const task = { ...fixture("t-laundry"), due: null, labels: ["errand"] };
  const { container } = renderRow(task);
  expect(childClasses(container.querySelector(".task-bottom"))).toEqual(["task-labels", "task-project"]);
});

it("the when group gets its own row above labels and project", () => {
  const { container } = renderRow(fixture("t-laundry"));
  const body = container.querySelector(".task-body");
  expect(childClasses(body)).toEqual(["task-title", "task-when", "task-bottom"]);
  expect(childClasses(container.querySelector(".task-bottom"))).toEqual(["task-labels", "task-project"]);
});

it("the project shows on every row, with its section apart so it can shorten first", () => {
  const { container } = renderRow(fixture("t-laundry"), {
    location: { project: "Home", section: "Chores" },
  });
  const project = container.querySelector(".task-project");
  expect(project?.textContent).toBe("Home / Chores");
  expect(project?.querySelector(".task-project-name")?.textContent).toBe("Home");
  expect(project?.querySelector(".task-project-section")?.textContent).toBe(" / Chores");
});

it("sub-tasks show X of Y with an icon for how far along they are", () => {
  const view = (done: number, total: number) => {
    const r = renderRow(fixture("t-laundry"), { progress: { done, total } });
    const sub = r.container.querySelector(".task-subtasks");
    const out = {
      text: sub?.querySelector(".task-count")?.textContent,
      kind: sub?.querySelector("[data-progress]")?.getAttribute("data-progress"),
      dash: sub?.querySelector("circle[stroke-dasharray]")?.getAttribute("stroke-dasharray") ?? null,
    };
    r.unmount();
    return out;
  };
  expect(view(0, 4)).toEqual({ text: "0 of 4", kind: "none", dash: null });
  expect(view(2, 5)).toEqual({ text: "2 of 5", kind: "some", dash: "241 603" });
  expect(view(3, 3)).toEqual({ text: "3 of 3", kind: "all", dash: null });
});

it("labels group by color, one tag each, tinted by that color, in added order", () => {
  const labels = [label("a", "red"), label("b", "red"), label("c", "blue")];
  const task = { ...fixture("t-laundry"), labels: ["c", "a", "b"] };
  const { container } = renderRow(task, { labels });
  const groups = [...container.querySelectorAll<HTMLElement>(".task-label-group")];
  expect(groups.map((g) => g.textContent)).toEqual(["c", "a, b"]);
  expect(groups.map((g) => g.querySelectorAll("svg").length)).toEqual([1, 1]);
  expect(groups.map((g) => g.style.getPropertyValue("--tint"))).toEqual([
    "var(--label-blue)",
    "var(--label-red)",
  ]);
});

it("a label with no record still shows, in the default group", () => {
  const task = { ...fixture("t-laundry"), labels: ["ghost"] };
  const { container } = renderRow(task, { labels: [] });
  const group = container.querySelector<HTMLElement>(".task-label-group");
  expect(group?.textContent).toBe("ghost");
  expect(group?.style.getPropertyValue("--tint")).toBe("var(--label-charcoal)");
});

it("beside a long project the when group never shortens, the project does", () => {
  const task = { ...fixture("t-laundry"), labels: [] };
  const { container } = renderRow(task, {
    location: { project: "A very long project name indeed", section: "A long section" },
  });
  expect(container.querySelector(".task-when")?.className).not.toContain("row-shrink");
  expect(container.querySelector(".task-project")?.className).toContain("row-shrink");
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
  expect(container.querySelector(".streak")?.textContent).toBe("4");
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

it("a completed row's long project can shorten", () => {
  const { container } = render(
    <CompletedRow
      task={fixture("t-laundry")}
      today={today}
      location="A long project / A long section"
      onChanged={() => {}}
    />,
    { wrapper },
  );
  expect(container.querySelector(".task-project")?.className).toContain("row-shrink");
});

it("a completed row reopens from its filled check", async () => {
  const onChanged = vi.fn();
  const { container } = render(
    <CompletedRow task={fixture("t-done")} today={today} location="Home" onChanged={onChanged} />,
    { wrapper },
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
      locations={new Map()}
      labels={[]}
      reminders={[]}
      onChanged={() => {}}
    />,
    { wrapper },
  );
  const head = container.querySelector(".task-card > .card-head");
  expect(head?.querySelector(".pname")?.textContent).toBe("Do laundry");
  expect(head?.querySelector(".card-flag")?.className).toBe("card-flag p1");
  expect(head?.querySelector(".task-count")?.textContent).toBe("0 of 1");
  expect([...container.querySelectorAll(".task-row .task-content")].map((e) => e.textContent)).toEqual([
    "Wash the sheets",
  ]);
  expect(container.querySelector(".task-project")?.textContent).toBe("Inbox");
});

it("the streak goal takes only digits", () => {
  const task = { ...fixture("t-laundry"), streak_enabled: true };
  if (task.due) task.due = { ...task.due, is_recurring: true };
  const { container } = renderRow(task);
  fireEvent.click(screen.getByTitle("Streak"));
  const goal = container.querySelector<HTMLInputElement>(".streak-panel input") as HTMLInputElement;
  expect(goal.inputMode).toBe("numeric");
  expect(goal.pattern).toBe("[0-9]*");
  fireEvent.change(goal, { target: { value: "1a2.-e3" } });
  expect(goal.value).toBe("123");
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

it("a row's body opens its task with its ancestors under it", () => {
  const { container } = renderRow(fixture("t-bag"));
  expect(opened()).toBe("|");
  fireEvent.click(container.querySelector(".task-body") as HTMLElement);
  expect(opened()).toBe("t-trip,t-bag|");
});

it("Add sub-task opens the task with its sub-task editor", () => {
  renderRow(fixture("t-laundry"));
  fireEvent.click(screen.getByTitle("Add sub-task"));
  expect(opened()).toBe("t-laundry|add");
});

it("a card head opens its task", () => {
  const { container } = render(
    <CardList
      cards={[
        {
          head: row(fixture("t-laundry"), { done: 0, total: 1 }),
          head_matches: false,
          rows: [row(fixture("t-sheets"))],
        },
      ]}
      today={today}
      locations={new Map()}
      labels={fx.labels}
      reminders={[]}
      onChanged={() => {}}
    />,
    { wrapper },
  );
  fireEvent.click(container.querySelector(".card-head") as HTMLElement);
  expect(opened()).toBe("t-laundry|");
});

it("a completed row's body opens its task", () => {
  const { container } = render(
    <CompletedRow task={fixture("t-done")} today={today} location="Home" onChanged={() => {}} />,
    { wrapper },
  );
  fireEvent.click(container.querySelector(".task-body") as HTMLElement);
  expect(opened()).toBe("t-done|");
});

const press = (container: HTMLElement) =>
  act(() => {
    container.querySelector(".task")?.dispatchEvent(new Event(ROW_PRESS));
  });
const taskMenu = () => {
  const menu = document.body.querySelector<HTMLElement>(".task-menu");
  return menu === null ? null : within(menu);
};

it("a long press lifts the row and opens its menu: add sub-task, delete", () => {
  const { container } = renderRow(fixture("t-laundry"));
  expect(taskMenu()).toBeNull();
  press(container);
  expect(taskEl(container)?.classList.contains("lifted")).toBe(true);
  expect(taskMenu()?.getByText("Add sub-task")).toBeTruthy();
  expect(taskMenu()?.getByText("Delete").closest(".menu-item")?.classList.contains("danger")).toBe(true);
  // Streak is for repeating tasks.
  expect(taskMenu()?.queryByText("Streak")).toBeNull();
});

const menuLabels = () =>
  [...(document.body.querySelectorAll(".task-menu .menu-label") ?? [])].map((e) => e.textContent);
const picker = () => {
  const el = document.body.querySelector<HTMLElement>(".move-picker");
  return el === null ? null : within(el);
};

it("the menu lists Add sub-task, Move to…, then Delete", () => {
  const { container } = renderRow(fixture("t-laundry"));
  press(container);
  expect(menuLabels()).toEqual(["Add sub-task", "Move to…", "Delete"]);
});

it("Move to… lists the projects and their sections, with where the task is checked", () => {
  const { container } = renderRow(fixture("t-groceries"));
  press(container);
  fireEvent.click(taskMenu()?.getByText("Move to…") as HTMLElement);
  expect(taskMenu()).toBeNull();
  const names = [...document.body.querySelectorAll(".move-picker .menu-label")].map((e) => e.textContent);
  expect(names).toEqual(["Inbox", "Home", "Chores", "Trip"]);
  const checked = [...document.body.querySelectorAll(".move-picker .menu-item.checked .menu-label")].map(
    (e) => e.textContent,
  );
  expect(checked).toEqual(["Home"]);
  expect(taskEl(container)?.classList.contains("lifted")).toBe(true);
});

it("picking a section moves the task into it, and picking a project moves it out", async () => {
  const { container, onChanged } = renderRow(fixture("t-groceries"));
  press(container);
  fireEvent.click(taskMenu()?.getByText("Move to…") as HTMLElement);
  fireEvent.click(picker()?.getByText("Chores") as HTMLElement);
  await tick(0);
  let saved = (await invoke("all_tasks", {})).find((t) => t.id === "t-groceries");
  expect([saved?.project_id, saved?.section_id]).toEqual(["p-home", "s-chores"]);
  expect(onChanged).toHaveBeenCalled();
  expect(picker()).toBeNull();
  expect(taskEl(container)?.classList.contains("lifted")).toBe(false);

  press(container);
  fireEvent.click(taskMenu()?.getByText("Move to…") as HTMLElement);
  fireEvent.click(picker()?.getByText("Trip") as HTMLElement);
  await tick(0);
  saved = (await invoke("all_tasks", {})).find((t) => t.id === "t-groceries");
  expect([saved?.project_id, saved?.section_id]).toEqual(["p-trip", null]);
});

it("Escape closes the Move to… list and puts the row back", () => {
  const { container } = renderRow(fixture("t-groceries"));
  press(container);
  fireEvent.click(taskMenu()?.getByText("Move to…") as HTMLElement);
  fireEvent.keyDown(document, { key: "Escape" });
  expect(picker()).toBeNull();
  expect(taskEl(container)?.classList.contains("lifted")).toBe(false);
});

it("the menu's Add sub-task opens the task's sub-task editor and closes the menu", () => {
  const { container } = renderRow(fixture("t-laundry"));
  press(container);
  fireEvent.click(taskMenu()?.getByText("Add sub-task") as HTMLElement);
  expect(opened()).toBe("t-laundry|add");
  expect(taskMenu()).toBeNull();
  expect(taskEl(container)?.classList.contains("lifted")).toBe(false);
});

it("a repeating task's menu has Streak, which opens the streak panel", () => {
  const task = fixture("t-laundry");
  if (task.due) task.due.is_recurring = true;
  const { container } = renderRow(task);
  press(container);
  fireEvent.click(taskMenu()?.getByText("Streak") as HTMLElement);
  expect(container.querySelector(".streak-panel")).not.toBeNull();
  expect(taskMenu()).toBeNull();
});

it("the menu's Delete deletes the task", async () => {
  const { container, onChanged } = renderRow(fixture("t-groceries"));
  press(container);
  fireEvent.click(taskMenu()?.getByText("Delete") as HTMLElement);
  await tick(0);
  expect((await invoke("all_tasks", {})).map((t) => t.id)).not.toContain("t-groceries");
  expect(onChanged).toHaveBeenCalled();
});

it("Escape closes the menu and puts the row back", () => {
  const { container } = renderRow(fixture("t-laundry"));
  press(container);
  fireEvent.keyDown(document, { key: "Escape" });
  expect(taskMenu()).toBeNull();
  expect(taskEl(container)?.classList.contains("lifted")).toBe(false);
});
