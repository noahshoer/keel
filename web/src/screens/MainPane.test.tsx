import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useState } from "react";
import { beforeAll, expect, it, vi } from "vitest";
import { useLoaded } from "../app/load";
import type { View } from "../app/view";
import { localToday } from "../clock";
import { invoke } from "../ipc/invoke";
import { TaskNavProvider } from "../taskview/nav";
import { loadCoreForTests } from "../test/wasm";
import { MainPane } from "./MainPane";

beforeAll(() => {
  loadCoreForTests();
});

/** The pane over the real loader, reloading on every change (as App does). */
// biome-ignore lint/style/useComponentExportOnlyModules: a test harness, never hot-reloaded.
function Harness({ view, showCompleted = false }: { view: View; showCompleted?: boolean }) {
  const [reloadKey, setReloadKey] = useState(0);
  const [completed, setCompleted] = useState(showCompleted);
  const { loaded } = useLoaded(view, completed, reloadKey);
  if (loaded === null) return null;
  return (
    <TaskNavProvider all={loaded.all}>
      <MainPane
        loaded={loaded}
        showCompleted={completed}
        onToggleCompleted={() => setCompleted((c) => !c)}
        onOpenDrawer={() => {}}
        motion="fade"
        swap={0}
        onChanged={() => setReloadKey((k) => k + 1)}
      />
    </TaskNavProvider>
  );
}

const show = async (view: View, showCompleted = false) => {
  const r = render(<Harness view={view} showCompleted={showCompleted} />);
  await waitFor(() => expect(r.container.querySelector(".view-title")).not.toBeNull());
  return r;
};
const titles = (c: HTMLElement) =>
  [...c.querySelectorAll(".section-title")].map((e) => e.firstChild?.textContent ?? "");
const contents = (c: HTMLElement) => [...c.querySelectorAll(".task-content")].map((e) => e.textContent);

it("today splits overdue from today, rows name their project", async () => {
  const { container } = await show({ kind: "today" });
  expect(container.querySelector(".view-title")?.textContent).toBe("Today");
  expect(titles(container)).toEqual(["Overdue", "Today"]);
  expect(contents(container)).toEqual(["File the taxes", "Do laundry", "Wash the sheets"]);
  // Classes Demo.swift and the sim checks query.
  expect(container.querySelectorAll(".task-row")).toHaveLength(3);
  const laundry = screen.getByText("Do laundry").closest(".task-row");
  expect(laundry?.querySelector(".task-count")?.textContent).toBe("0/1");
  expect(laundry?.querySelector(".task-project")?.textContent).toBe("Home / Chores");
});

it("upcoming shows the overdue task first, then a group per day", async () => {
  const { container } = await show({ kind: "upcoming" });
  expect(container.querySelector(".view-sub")?.textContent).toBe("Next 14 days");
  expect(titles(container)[0]).toBe("Overdue");
  expect(titles(container)).toHaveLength(3);
  expect(container.querySelector(".view-toggle")).toBeNull();
  expect(contents(container)[0]).toBe("File the taxes");
});

it("a project shows unsectioned tasks, then its sections with counts", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  const blocks = container.querySelectorAll(".section-block");
  expect(blocks).toHaveLength(1);
  const head = blocks[0]?.querySelector(".section-title");
  expect(head?.firstChild?.textContent).toBe("Chores");
  // Top-level tasks only: the laundry card (its sub-task is counted, not listed).
  expect(head?.querySelector(".muted")?.textContent).toBe("1");
  expect(contents(container)).toEqual(["Buy groceries", "Do laundry"]);
  expect(container.querySelector(".task-project")).toBeNull();
  expect(container.querySelector(".add-section-row")).not.toBeNull();
});

it("renaming a section changes its head", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  fireEvent.click(screen.getByTitle("Rename section"));
  const input = container.querySelector<HTMLInputElement>(".section-rename input");
  expect(document.activeElement).toBe(input);
  fireEvent.change(input as HTMLInputElement, { target: { value: "Weekly" } });
  fireEvent.submit(input as HTMLInputElement);
  await waitFor(() => expect(titles(container)).toContain("Weekly"));
});

it("deleting a section asks first", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  fireEvent.click(screen.getByTitle("Delete section"));
  expect(container.querySelector(".confirm-text")?.textContent).toBe(
    "Delete section “Chores” and its 1 task?",
  );
  fireEvent.click(screen.getByText("Cancel"));
  expect(container.querySelector(".confirm-box")).toBeNull();
  fireEvent.click(screen.getByTitle("Delete section"));
  fireEvent.click(screen.getByText("Delete"));
  await waitFor(() => expect(container.querySelector(".section-block")).toBeNull());
  expect(await invoke("list_sections", { project_id: "p-home" })).toEqual([]);
});

it("adding a section from the divider", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  fireEvent.click(screen.getByText("Add section"));
  const input = screen.getByPlaceholderText("Name this section, then Enter");
  expect(document.activeElement).toBe(input);
  fireEvent.keyDown(input, { key: "Escape" });
  expect(container.querySelector(".add-section-row")).not.toBeNull();
  fireEvent.click(screen.getByText("Add section"));
  const again = screen.getByPlaceholderText("Name this section, then Enter");
  fireEvent.change(again, { target: { value: "Garden" } });
  fireEvent.submit(again);
  await waitFor(() => expect(titles(container)).toContain("Garden"));
});

it("a filter with two comma groups titles both", async () => {
  const { container } = await show({ kind: "filter", id: "f", name: "Two", query: "p1, today" });
  expect(container.querySelector(".view-sub")?.className).toBe("view-sub mono");
  expect([...container.querySelectorAll(".section-title .mono")].map((e) => e.textContent)).toEqual([
    "p1",
    "today",
  ]);
  const single = await show({ kind: "filter", id: "f-p1", name: "Important", query: "p1" });
  expect(single.container.querySelector(".section-title")).toBeNull();
});

it("a saved filter that can't run says why", async () => {
  const { container } = await show({ kind: "filter", id: "f-bad", name: "Broken", query: "p1 &" });
  expect(container.querySelector(".filter-problem.big-problem")?.textContent).toBe(
    "This filter can’t run: expected a term after '&'",
  );
  expect(container.querySelector(".empty")).toBeNull();
});

it("show completed lists the view's completed tasks", async () => {
  const { container } = await show({ kind: "today" });
  const toggle = container.querySelector<HTMLButtonElement>(".view-toggle");
  expect(toggle?.textContent).toBe("Show completed");
  fireEvent.click(toggle as HTMLButtonElement);
  await waitFor(() => expect(titles(container)).toContain("Completed"));
  expect(container.querySelector(".view-toggle")?.textContent).toBe("Hide completed");
  const done = container.querySelector(".task.completed");
  expect(done && within(done as HTMLElement).getByText("Take out the bins")).toBeTruthy();
});

it("an empty inbox says so", async () => {
  await invoke("dispatch", { cmd: { uuid: "u", action: { type: "task_delete", id: "t-inbox" } } });
  await invoke("dispatch", { cmd: { uuid: "v", action: { type: "task_delete", id: "t-taxes" } } });
  const { container } = await show({ kind: "inbox" });
  expect(container.querySelector(".empty .big")?.textContent).toBe("No tasks yet.");
});

it("completing a row removes it once the view reloads", async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  try {
    const { container } = await show({ kind: "today" });
    const row = screen.getByText("File the taxes").closest(".task-row");
    fireEvent.click(row?.querySelector("button.check") as HTMLButtonElement);
    await act(async () => vi.advanceTimersByTimeAsync(800));
    await waitFor(() => expect(contents(container)).not.toContain("File the taxes"));
    expect(titles(container)).toEqual([]);
  } finally {
    vi.useRealTimers();
  }
});

const allTasks = () => invoke("all_tasks", {});
const titleField = () =>
  screen.getByPlaceholderText(
    "Task name — try “Call mom tomorrow 5pm p1 #Home @phone”",
  ) as HTMLTextAreaElement;

it("+ Add task opens the editor focused; Enter adds a task due today and stays open", async () => {
  const { container } = await show({ kind: "today" });
  const row = container.querySelector(".add-task-row.view-add") as HTMLElement;
  fireEvent.click(row);
  const title = titleField();
  expect(document.activeElement).toBe(title);
  fireEvent.change(title, { target: { value: "Water plants" } });
  await act(async () => {
    fireEvent.keyDown(title, { key: "Enter" });
  });
  await waitFor(async () => expect((await allTasks()).some((t) => t.content === "Water plants")).toBe(true));
  const added = (await allTasks()).find((t) => t.content === "Water plants");
  expect(added?.due?.date).toBe(localToday());
  expect(titleField().value).toBe("");
});

it("a project's add row puts the task in it and lists it", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  // The project's own row comes before its sections; each section has one too.
  const rows = container.querySelectorAll(".add-task-row");
  expect(rows).toHaveLength(2);
  expect(rows[0]?.classList.contains("view-add")).toBe(true);
  fireEvent.click(rows[0] as HTMLElement);
  fireEvent.change(titleField(), { target: { value: "Fix the gate" } });
  await act(async () => {
    fireEvent.keyDown(titleField(), { key: "Enter" });
  });
  await waitFor(() => expect(contents(container)).toContain("Fix the gate"));
  expect((await allTasks()).find((t) => t.content === "Fix the gate")?.project_id).toBe("p-home");
});

it("a section's add row adds into the section", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  const block = container.querySelector(".section-block") as HTMLElement;
  fireEvent.click(within(block).getByText("Add task"));
  fireEvent.change(titleField(), { target: { value: "Iron shirts" } });
  await act(async () => {
    fireEvent.keyDown(titleField(), { key: "Enter" });
  });
  await waitFor(async () => {
    const t = (await allTasks()).find((x) => x.content === "Iron shirts");
    expect(t?.section_id).toBe("s-chores");
  });
});

it("a label view's add carries the label", async () => {
  const { container } = await show({ kind: "label", name: "errand" });
  fireEvent.click(container.querySelector(".add-task-row") as HTMLElement);
  fireEvent.change(titleField(), { target: { value: "Post the letter" } });
  await act(async () => {
    fireEvent.keyDown(titleField(), { key: "Enter" });
  });
  await waitFor(async () =>
    expect((await allTasks()).find((t) => t.content === "Post the letter")?.labels).toEqual(["errand"]),
  );
});

it("Cancel collapses the editor; Search has no add row", async () => {
  const { container, unmount } = await show({ kind: "inbox" });
  fireEvent.click(container.querySelector(".add-task-row") as HTMLElement);
  fireEvent.click(screen.getByText("Cancel"));
  expect(container.querySelector(".editor")).toBeNull();
  expect(container.querySelector(".add-task-row")).not.toBeNull();
  unmount();
  const search = await show({ kind: "search" });
  expect(search.container.querySelector(".add-task-row")).toBeNull();
});
