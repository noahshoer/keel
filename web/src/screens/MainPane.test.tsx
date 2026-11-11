import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useState } from "react";
import { beforeAll, expect, it, vi } from "vitest";
import { useLoaded } from "../app/load";
import type { View } from "../app/view";
import type { OrderLevel, SortField } from "../bindings";
import { localToday } from "../clock";
import * as core from "../core";
import { TASK_DROP } from "../gestures/drag";
import { ROW_PRESS } from "../gestures/long-press";
import { addLabel, arrangeView, dispatchAction, renameSection } from "../ipc/actions";
import { invoke } from "../ipc/invoke";
import { ShowSubtasksContext } from "../lists/showSubtasks";
import { TaskNavProvider } from "../taskview/nav";
import { loadCoreForTests } from "../test/wasm";
import { MainPane } from "./MainPane";

beforeAll(() => {
  loadCoreForTests();
});

/** The pane over the real loader, reloading on every change (as App does). */
// biome-ignore lint/style/useComponentExportOnlyModules: a test harness, never hot-reloaded.
function Harness({
  view,
  showCompleted = false,
  showSubtasks = false,
}: {
  view: View;
  showCompleted?: boolean;
  showSubtasks?: boolean;
}) {
  const [reloadKey, setReloadKey] = useState(0);
  const [completed, setCompleted] = useState(showCompleted);
  const { loaded } = useLoaded(view, completed, reloadKey);
  if (loaded === null) return null;
  return (
    <ShowSubtasksContext.Provider value={{ show: showSubtasks, setShow: () => {} }}>
      <TaskNavProvider all={loaded.all}>
        <MainPane
          loaded={loaded}
          showCompleted={completed}
          onToggleCompleted={() => setCompleted((c) => !c)}
          onOpenDrawer={() => {}}
          onLeaveSearch={() => {}}
          setView={() => {}}
          renameView={() => {}}
          holdName={() => {}}
          onOpenSearch={() => {}}
          motion="fade"
          swap={0}
          onChanged={() => setReloadKey((k) => k + 1)}
        />
      </TaskNavProvider>
    </ShowSubtasksContext.Provider>
  );
}

const show = async (view: View, showCompleted = false, showSubtasks = false) => {
  const r = render(<Harness view={view} showCompleted={showCompleted} showSubtasks={showSubtasks} />);
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
  expect(contents(container)).toEqual(["File the taxes", "Do laundry"]);
  // Classes Demo.swift and the sim checks query.
  expect(container.querySelectorAll(".task-row")).toHaveLength(2);
  const laundry = screen.getByText("Do laundry").closest(".task-row");
  expect(laundry?.querySelector(".task-count")?.textContent).toBe("0 of 1");
  expect(laundry?.querySelector(".task-project")?.textContent).toBe("Home / Chores");
});

it("the setting shows sub-tasks under their parents, in dated and project lists", async () => {
  const today = await show({ kind: "today" }, false, true);
  expect(contents(today.container)).toEqual(["File the taxes", "Do laundry", "Wash the sheets"]);
  today.unmount();
  const off = await show({ kind: "project", id: "p-trip", name: "Trip" });
  await screen.findByText("Pack for the trip");
  expect(contents(off.container)).toEqual(["Pack for the trip"]);
  off.unmount();
  const on = await show({ kind: "project", id: "p-trip", name: "Trip" }, false, true);
  await screen.findByText("Print the tickets");
  expect(contents(on.container)).toEqual(["Pack for the trip", "Pack the bag", "Socks", "Print the tickets"]);
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
  expect(container.querySelector(".task-project")?.textContent).toContain("Home");
  // Always drawn, like "+ Add task": a plus and its label, no hover-only divider.
  const add = container.querySelector(".add-section-row");
  expect(add?.textContent).toBe("Add section");
  expect(add?.querySelector(".plus")).not.toBeNull();
  expect(container.querySelector(".add-section-line")).toBeNull();
});

const popup = () => document.body.querySelector<HTMLElement>(".row-menu");

it("a section's ••• opens the project's popup; Edit renames it in place", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  expect(container.querySelector(".section-actions")).toBeNull();
  fireEvent.click(screen.getByLabelText("Section actions"));
  expect(within(popup() as HTMLElement).queryByText("Add section")).toBeNull();
  fireEvent.click(within(popup() as HTMLElement).getByText("Edit"));
  expect(popup()).toBeNull();
  const input = container.querySelector<HTMLInputElement>(".section-name input");
  expect(document.activeElement).toBe(input);
  fireEvent.change(input as HTMLInputElement, { target: { value: "Weekly" } });
  fireEvent.submit(input as HTMLInputElement);
  await waitFor(() => expect(titles(container)).toContain("Weekly"));
});

it("deleting a section asks first, in the popup", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  fireEvent.click(screen.getByLabelText("Section actions"));
  fireEvent.click(within(popup() as HTMLElement).getByText("Delete"));
  expect(popup()?.querySelector(".confirm-text")?.textContent).toBe(
    "Delete section “Chores” and its 1 task?",
  );
  expect(container.querySelector(".confirm-box")).toBeNull();
  fireEvent.click(within(popup() as HTMLElement).getByText("Cancel"));
  expect(popup()).toBeNull();
  expect(container.querySelector(".section-block")).not.toBeNull();
  fireEvent.click(screen.getByLabelText("Section actions"));
  fireEvent.click(within(popup() as HTMLElement).getByText("Delete"));
  fireEvent.click(within(popup() as HTMLElement).getByText("Delete", { selector: "button.btn-danger" }));
  await waitFor(() => expect(container.querySelector(".section-block")).toBeNull());
  expect(await invoke("list_sections", { project_id: "p-home" })).toEqual([]);
});

it("the name fields of a new and a renamed section are the header's own style, not a boxed input", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  fireEvent.click(screen.getByText("Add section"));
  const add = screen.getByPlaceholderText("Name this section, then Enter");
  expect(add.className).toBe("section-name-input");
  expect(add.closest("form")?.className).toBe("section-name section-name-new");
  expect(container.querySelector(".project-input")).toBeNull();
  fireEvent.keyDown(add, { key: "Escape" });
  fireEvent.click(screen.getByLabelText("Section actions"));
  fireEvent.click(within(popup() as HTMLElement).getByText("Edit"));
  const rename = container.querySelector(".section-name input");
  expect(rename?.className).toBe("section-name-input");
  expect(rename?.closest("form")?.className).toBe("section-name");
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

it("a long press or right click on a section's header opens the same popup", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  act(() => {
    container.querySelector(".section-row")?.dispatchEvent(new Event(ROW_PRESS));
  });
  expect(within(popup() as HTMLElement).getByText("Edit")).toBeTruthy();
  expect(within(popup() as HTMLElement).getByText("Delete")).toBeTruthy();
});

it("the project's ••• has Add section first, which opens the name field", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  fireEvent.click(screen.getByLabelText("More"));
  const entries = [...(popup() as HTMLElement).querySelectorAll(".menu-label")].map((e) => e.textContent);
  expect(entries).toEqual(["Add section", "Edit", "Delete"]);
  fireEvent.click(within(popup() as HTMLElement).getByText("Add section"));
  expect(popup()).toBeNull();
  const input = screen.getByPlaceholderText("Name this section, then Enter");
  expect(document.activeElement).toBe(input);
  fireEvent.change(input, { target: { value: "Garden" } });
  fireEvent.submit(input);
  await waitFor(() => expect(titles(container)).toContain("Garden"));
});

it("a label or filter page's ••• has no Add section", async () => {
  await show({ kind: "label", name: "errand" });
  fireEvent.click(screen.getByLabelText("More"));
  const entries = [...(popup() as HTMLElement).querySelectorAll(".menu-label")].map((e) => e.textContent);
  expect(entries).toEqual(["Edit", "Delete"]);
});

it("the room for the floating add button is at the end of the page, not between the view's add row and its sections", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  const spacers = container.querySelectorAll(".fab-spacer");
  expect(spacers).toHaveLength(1);
  const spacer = spacers[0] as Element;
  const follows = (el: Element | null) =>
    el !== null && Boolean(el.compareDocumentPosition(spacer) & Node.DOCUMENT_POSITION_FOLLOWING);
  expect(follows(container.querySelector(".view-add"))).toBe(true);
  expect(follows(container.querySelector(".section-block"))).toBe(true);
  expect(follows(container.querySelector(".add-section-row"))).toBe(true);
});

it("tapping outside the Add section field closes it and brings the row back, dropping what was typed", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  fireEvent.click(screen.getByText("Add section"));
  const input = screen.getByPlaceholderText("Name this section, then Enter");
  fireEvent.change(input, { target: { value: "Garden" } });
  // Inside it: it stays.
  fireEvent.click(input);
  expect(screen.queryByPlaceholderText("Name this section, then Enter")).not.toBeNull();
  fireEvent.click(container.querySelector(".view-title") as HTMLElement);
  expect(screen.queryByPlaceholderText("Name this section, then Enter")).toBeNull();
  expect(container.querySelector(".add-section-row")?.textContent).toBe("Add section");
  fireEvent.click(screen.getByText("Add section"));
  expect((screen.getByPlaceholderText("Name this section, then Enter") as HTMLInputElement).value).toBe("");
  expect(titles(container)).not.toContain("Garden");
});

const drop = (zone: Element | null, taskId: string) =>
  act(() => {
    zone?.dispatchEvent(new CustomEvent(TASK_DROP, { bubbles: true, detail: { taskId } }));
  });

it("a project's sections and its unsectioned tasks are drop zones that move a task in and out", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  const zones = [...container.querySelectorAll("[data-drop-zone]:not([data-drop-new])")];
  expect(
    zones.map((z) => [
      z.getAttribute("data-drop-project"),
      z.getAttribute("data-drop-section"),
      z.getAttribute("data-drop-name"),
    ]),
  ).toEqual([
    ["p-home", "", "Home"],
    ["p-home", "s-chores", "Chores"],
  ]);
  drop(zones[1] ?? null, "t-groceries");
  await waitFor(async () => {
    const t = (await allTasks()).find((x) => x.id === "t-groceries");
    expect(t?.section_id).toBe("s-chores");
  });
  await waitFor(() =>
    expect(container.querySelector(".section-block")?.textContent).toContain("Buy groceries"),
  );
  drop(container.querySelectorAll("[data-drop-zone]")[0] ?? null, "t-groceries");
  await waitFor(async () => {
    const t = (await allTasks()).find((x) => x.id === "t-groceries");
    expect(t?.section_id).toBeNull();
  });
});

it("a dragged task dropped on Add section opens the name field; naming it creates the section and moves the task in", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  const row = container.querySelector(".add-section-row");
  expect(row?.hasAttribute("data-drop-zone")).toBe(true);
  expect(row?.hasAttribute("data-drop-new")).toBe(true);
  expect(row?.getAttribute("data-drop-project")).toBe("p-home");
  drop(row, "t-groceries");
  const input = screen.getByPlaceholderText("Name the new section, then Enter");
  expect(document.activeElement).toBe(input);
  // Nothing has moved yet.
  expect((await allTasks()).find((t) => t.id === "t-groceries")?.section_id).toBeNull();
  fireEvent.change(input, { target: { value: "Garden" } });
  fireEvent.submit(input);
  await waitFor(async () => {
    const garden = (await invoke("list_sections", { project_id: "p-home" })).find((s) => s.name === "Garden");
    expect(garden).toBeDefined();
    expect((await allTasks()).find((t) => t.id === "t-groceries")?.section_id).toBe(garden?.id);
  });
  await waitFor(() => expect(titles(container)).toContain("Garden"));
});

it("a field opened by a drop that is closed again leaves the task where it was and makes no section", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  drop(container.querySelector(".add-section-row"), "t-groceries");
  fireEvent.keyDown(screen.getByPlaceholderText("Name the new section, then Enter"), { key: "Escape" });
  expect(container.querySelector(".add-section-row")).not.toBeNull();
  // A plain tap on the row afterwards is an ordinary Add section, with nothing waiting to move.
  fireEvent.click(screen.getByText("Add section"));
  const input = screen.getByPlaceholderText("Name this section, then Enter");
  fireEvent.change(input, { target: { value: "Later" } });
  fireEvent.submit(input);
  await waitFor(() => expect(titles(container)).toContain("Later"));
  expect((await allTasks()).find((t) => t.id === "t-groceries")?.section_id).toBeNull();
});

const waitingCard = (container: HTMLElement) => container.querySelector<HTMLElement>(".move-preview");
const waitingMark = (container: HTMLElement, id: string) =>
  container.querySelector(`.task[data-menu-id="${id}"]`)?.hasAttribute("data-move-waiting") ?? false;

it("while the new section is named, a copy of the dropped task waits under the field and the original is marked", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  expect(waitingCard(container)).toBeNull();
  drop(container.querySelector(".add-section-row"), "t-groceries");
  expect(waitingCard(container)?.textContent).toContain("Buy groceries");
  expect(waitingCard(container)?.textContent).toContain("Moves here");
  // Under the field, inside the form, so a tap on it doesn't close the field.
  expect(container.querySelector(".section-name-new")?.contains(waitingCard(container))).toBe(true);
  expect(waitingMark(container, "t-groceries")).toBe(true);
  const input = screen.getByPlaceholderText("Name the new section, then Enter");
  fireEvent.change(input, { target: { value: "Garden" } });
  fireEvent.submit(input);
  await waitFor(() => expect(titles(container)).toContain("Garden"));
  expect(waitingCard(container)).toBeNull();
  expect(waitingMark(container, "t-groceries")).toBe(false);
});

it("closing the field takes the waiting copy away and unmarks the original", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  drop(container.querySelector(".add-section-row"), "t-groceries");
  fireEvent.keyDown(screen.getByPlaceholderText("Name the new section, then Enter"), { key: "Escape" });
  expect(waitingCard(container)).toBeNull();
  expect(waitingMark(container, "t-groceries")).toBe(false);
  drop(container.querySelector(".add-section-row"), "t-groceries");
  fireEvent.click(container.querySelector(".view-title") as HTMLElement);
  expect(waitingCard(container)).toBeNull();
  expect(waitingMark(container, "t-groceries")).toBe(false);
  // A plain Add section has no copy.
  fireEvent.click(screen.getByText("Add section"));
  expect(waitingCard(container)).toBeNull();
});

it("a task is draggable at the top level only, and a project without sections has only Add section to drop on", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  const draggable = [...container.querySelectorAll(".task[data-draggable]")].map((t) =>
    t.getAttribute("data-menu-id"),
  );
  expect(draggable).toContain("t-groceries");
  expect(draggable).not.toContain("t-sheets");
  // With no sections, the only place to drop is Add section: it starts the first one.
  const none = await show({ kind: "project", id: "p-trip", name: "Trip" });
  const zones = [...none.container.querySelectorAll("[data-drop-zone]")];
  expect(zones.map((z) => z.className)).toEqual(["add-section-row"]);
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

it("a saved filter naming a missing project says so and still lists the rest", async () => {
  const { container } = await show({ kind: "filter", id: "f-gone", name: "Gone", query: "#Gone, today" });
  const line = container.querySelector(".filter-problem.view-problem");
  expect(line?.textContent).toBe("No project named “Gone”.");
  expect(container.querySelector(".big-problem")).toBeNull();
  expect([...container.querySelectorAll(".section-title .mono")].map((e) => e.textContent)).toEqual([
    "#Gone",
    "today",
  ]);
  expect(container.querySelectorAll(".task-row").length).toBeGreaterThan(0);
  // A query whose names all exist has no such line.
  const fine = await show({ kind: "filter", id: "f-p1", name: "Important", query: "p1" });
  expect(fine.container.querySelector(".view-problem")).toBeNull();
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

it("the completed toggle names itself and its state when only its icon shows", async () => {
  const { container } = await show({ kind: "today" });
  const toggle = container.querySelector<HTMLButtonElement>(".view-toggle") as HTMLButtonElement;
  expect(toggle.querySelector(".view-toggle-label")?.textContent).toBe("Show completed");
  expect(toggle.getAttribute("aria-label")).toBe("Show completed");
  expect(toggle.getAttribute("aria-pressed")).toBe("false");
  fireEvent.click(toggle);
  await waitFor(() => expect(toggle.getAttribute("aria-pressed")).toBe("true"));
  expect(toggle.getAttribute("aria-label")).toBe("Hide completed");
});

it("a different view opens at the top with the full title", async () => {
  const { container, rerender } = await show({ kind: "today" });
  const list = container.querySelector(".list-scroll") as HTMLElement;
  const head = container.querySelector(".view-head") as HTMLElement;
  list.scrollTop = 400;
  head.style.setProperty("--list-fade", "1");
  rerender(<Harness view={{ kind: "inbox" }} />);
  await waitFor(() => expect(container.querySelector(".view-title")?.textContent).toBe("Inbox"));
  expect(list.scrollTop).toBe(0);
  expect(head.style.getPropertyValue("--list-fade")).toBe("0");
});

it("the same view reloading keeps its place", async () => {
  const { container } = await show({ kind: "today" });
  const list = container.querySelector(".list-scroll") as HTMLElement;
  list.scrollTop = 400;
  fireEvent.click(container.querySelector(".view-toggle") as HTMLElement);
  await waitFor(() => expect(titles(container)).toContain("Completed"));
  expect(list.scrollTop).toBe(400);
});

const scrollTo = (list: HTMLElement, top: number) => {
  list.scrollTop = top;
  fireEvent.scroll(list);
};

it("coming back from Search keeps the list's place", async () => {
  const { container, rerender } = await show({ kind: "today" });
  const list = container.querySelector(".list-scroll") as HTMLElement;
  scrollTo(list, 400);
  rerender(<Harness view={{ kind: "search" }} />);
  await waitFor(() => expect(container.querySelector(".view-title")?.textContent).toBe("Search"));
  expect(list.scrollTop).toBe(0);
  rerender(<Harness view={{ kind: "today" }} />);
  await waitFor(() => expect(container.querySelector(".view-title")?.textContent).toBe("Today"));
  expect(list.scrollTop).toBe(400);
});

it("going from Search to a different view opens it at the top", async () => {
  const { container, rerender } = await show({ kind: "today" });
  const list = container.querySelector(".list-scroll") as HTMLElement;
  scrollTo(list, 400);
  rerender(<Harness view={{ kind: "search" }} />);
  await waitFor(() => expect(container.querySelector(".view-title")?.textContent).toBe("Search"));
  rerender(<Harness view={{ kind: "inbox" }} />);
  await waitFor(() => expect(container.querySelector(".view-title")?.textContent).toBe("Inbox"));
  expect(list.scrollTop).toBe(0);
});

it("renaming the project being viewed keeps its place", async () => {
  const { container, rerender } = await show({ kind: "project", id: "p-home", name: "Home" });
  const list = container.querySelector(".list-scroll") as HTMLElement;
  scrollTo(list, 400);
  rerender(<Harness view={{ kind: "project", id: "p-home", name: "House" }} />);
  await waitFor(() => expect(container.querySelector(".view-title")?.textContent).toBe("House"));
  expect(list.scrollTop).toBe(400);
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

// Arranged pages (issue #7).
const homePlace = { kind: "project", id: "p-home" } as const;
const arrangement = (a: Partial<{ filter: string; groups: SortField[]; order: OrderLevel[] }>) => ({
  filter: "",
  groups: [],
  order: [],
  ...a,
});
const addTask = (content: string, extra: Record<string, unknown> = {}) =>
  dispatchAction({ type: "task_add", project_id: "p-home", content, ...extra } as never);

it("untouched page is unchanged", async () => {
  const arrangeSpy = vi.spyOn(core, "arrange");
  try {
    const today = await show({ kind: "today" });
    expect(today.container.querySelector(".arrange-sub, .arrange-empty")).toBeNull();
    today.unmount();
    const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
    expect(container.querySelector(".arrange-sub")).toBeNull();
    expect(container.querySelector(".arrange-empty")).toBeNull();
    expect(container.querySelectorAll(".add-section-row")).toHaveLength(1);
    expect(container.querySelectorAll(".add-task-row")).toHaveLength(2);
    expect(arrangeSpy).not.toHaveBeenCalled();
  } finally {
    arrangeSpy.mockRestore();
  }
});

it("order only keeps sections", async () => {
  await dispatchAction({ type: "task_update", id: "t-laundry", priority: "None" } as never);
  await addTask("Urgent chore", { section_id: "s-chores", priority: "P1" });
  await arrangeView(homePlace, arrangement({ order: [{ field: "priority", reversed: false }] }));
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  expect(titles(container)).toEqual(["Chores"]);
  const block = container.querySelector(".section-block") as HTMLElement;
  expect(block.querySelector(".task-content")?.textContent).toBe("Urgent chore");
});

it("headings replace sections", async () => {
  await arrangeView(homePlace, arrangement({ groups: ["label"] }));
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  expect(titles(container)).toEqual(["errand", "No label"]);
  expect(container.querySelector(".add-section-row")).toBeNull();
  expect(container.querySelector(".section-block")).toBeNull();
  expect(container.querySelectorAll(".add-task-row")).toHaveLength(1);
});

it("today grouped like Noah's example", async () => {
  await addLabel("phone");
  const [chores] = await invoke("list_sections", { project_id: "p-home" });
  await renameSection(chores?.id as string, "Kitchen");
  await dispatchAction({ type: "task_update", id: "t-laundry", labels: ["errand", "phone"] } as never);
  await arrangeView({ kind: "today" }, arrangement({ groups: ["project", "section", "label"] }));
  const { container } = await show({ kind: "today" });
  expect(titles(container)).toContain("Home › Kitchen");
  expect([...container.querySelectorAll(".arrange-sub")].map((e) => e.textContent)).toEqual([
    "No label",
    "errand",
    "phone",
    "No label",
  ]);
});

it("two-label task shows twice and leaves twice", async () => {
  const errors = vi.spyOn(console, "error").mockImplementation(() => {});
  vi.useFakeTimers({ shouldAdvanceTime: true });
  try {
    await addLabel("phone");
    await dispatchAction({ type: "task_update", id: "t-laundry", labels: ["errand", "phone"] } as never);
    await arrangeView(homePlace, arrangement({ groups: ["label"] }));
    const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
    const laundry = () => contents(container).filter((c) => c === "Do laundry");
    expect(laundry()).toHaveLength(2);
    const row = screen.getAllByText("Do laundry")[0]?.closest(".task-row");
    fireEvent.click(row?.querySelector("button.check") as HTMLButtonElement);
    await act(async () => vi.advanceTimersByTimeAsync(800));
    await waitFor(() => expect(laundry()).toHaveLength(0));
    expect(errors.mock.calls.map((c) => String(c[0])).filter((m) => m.includes("same key"))).toEqual([]);
  } finally {
    vi.useRealTimers();
    errors.mockRestore();
  }
});

it("the menu offers Add section unless headings replace sections", async () => {
  const home = { kind: "project", id: "p-home", name: "Home" } as const;
  const first = await show(home);
  fireEvent.click(screen.getByLabelText("More"));
  expect(within(popup() as HTMLElement).queryByText("Add section")).not.toBeNull();
  first.unmount();
  await arrangeView(homePlace, arrangement({ order: [{ field: "priority", reversed: false }] }));
  const ordered = await show(home);
  fireEvent.click(screen.getByLabelText("More"));
  expect(within(popup() as HTMLElement).queryByText("Add section")).not.toBeNull();
  ordered.unmount();
  await arrangeView(homePlace, arrangement({ groups: ["label"] }));
  await show(home);
  fireEvent.click(screen.getByLabelText("More"));
  expect(within(popup() as HTMLElement).queryByText("Add section")).toBeNull();
});

it("nothing matches", async () => {
  await arrangeView(homePlace, arrangement({ filter: "p2" }));
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  expect(screen.getByText("No tasks match this filter.")).not.toBeNull();
  expect(container.querySelectorAll(".add-task-row")).toHaveLength(1);
  expect(container.querySelector(".empty")).toBeNull();
});

it("upcoming order only keeps days", async () => {
  await arrangeView({ kind: "upcoming" }, arrangement({ order: [{ field: "priority", reversed: false }] }));
  const { container } = await show({ kind: "upcoming" });
  const plain = titles(container);
  expect(plain[0]).toBe("Overdue");
  expect(plain).toHaveLength(3);
  expect(container.querySelector(".arrange-sub")).toBeNull();
});

// Header icons and chips (issue #7).
it("the filter and sort icons are on lists, not on Search", async () => {
  const search = await show({ kind: "search" });
  expect(screen.queryByLabelText("Filter")).toBeNull();
  expect(screen.queryByLabelText("Sort")).toBeNull();
  search.unmount();
  const { container } = await show({ kind: "today" });
  expect(screen.getByLabelText("Filter").getAttribute("aria-pressed")).toBe("false");
  expect(screen.getByLabelText("Sort").getAttribute("aria-pressed")).toBe("false");
  expect(container.querySelector(".arrange-btn.on")).toBeNull();
  expect(container.querySelector(".arrange-bar")).toBeNull();
});

it("a filter shows its chip and lights the button; clearing keeps the sort", async () => {
  const order = [{ field: "priority", reversed: false }] as OrderLevel[];
  await arrangeView(homePlace, arrangement({ filter: "p1", order }));
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  const texts = [...container.querySelectorAll(".arrange-chip-text")].map((e) => e.textContent);
  expect(texts).toEqual(["p1", core.arrangementSummary(arrangement({ order }))]);
  expect(container.querySelector(".arrange-chip.problem")).toBeNull();
  expect(screen.getByLabelText("Filter").getAttribute("aria-pressed")).toBe("true");
  expect(screen.getByLabelText("Sort").getAttribute("aria-pressed")).toBe("true");
  expect(container.querySelector(".list-body")?.firstElementChild?.className).toBe("arrange-bar");
  fireEvent.click(screen.getByLabelText("Clear filter"));
  await waitFor(() => expect(screen.queryByLabelText("Clear filter")).toBeNull());
  expect(screen.getByLabelText("Reset sort")).not.toBeNull();
  expect(screen.getByLabelText("Filter").getAttribute("aria-pressed")).toBe("false");
});

it("resetting the sort keeps the filter", async () => {
  await arrangeView(homePlace, arrangement({ filter: "p1", groups: ["label"] }));
  await show({ kind: "project", id: "p-home", name: "Home" });
  fireEvent.click(screen.getByLabelText("Reset sort"));
  await waitFor(() => expect(screen.queryByLabelText("Reset sort")).toBeNull());
  expect(screen.getByLabelText("Clear filter")).not.toBeNull();
  expect(screen.getByLabelText("Sort").getAttribute("aria-pressed")).toBe("false");
});

it("a broken remembered filter marks the chip and the list still shows every task", async () => {
  await arrangeView({ kind: "today" }, arrangement({ filter: "p1 &" }));
  const { container } = await show({ kind: "today" });
  const chip = container.querySelector(".arrange-chip") as HTMLElement;
  expect(chip.classList.contains("problem")).toBe(true);
  expect(chip.title).not.toBe("");
  expect(contents(container)).toEqual(["File the taxes", "Do laundry"]);
});

// The filter panel (issue #7).
it("typing a filter narrows the list behind the panel before anything is saved", async () => {
  await dispatchAction({ type: "task_update", id: "t-laundry", priority: "None" } as never);
  await addTask("Urgent chore", { priority: "P1" });
  const sent: string[] = [];
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  const before = contents(container);
  expect(before).toContain("Urgent chore");
  expect(before.length).toBeGreaterThan(1);
  const real = window.__TAURI__?.core.invoke;
  if (real && window.__TAURI__) {
    window.__TAURI__.core.invoke = (cmd, args) => {
      if (cmd === "dispatch") sent.push(cmd);
      return real(cmd, args);
    };
  }
  fireEvent.click(screen.getByLabelText("Filter"));
  const field = await screen.findByLabelText("Query");
  fireEvent.change(field, { target: { value: "p1" } });
  await waitFor(() => expect(contents(container)).toEqual(["Urgent chore"]));
  expect(sent).toEqual([]);
  expect(screen.getByText(/^1 of \d+ tasks match$/)).not.toBeNull();
  // Closing saves it.
  fireEvent.keyDown(document, { key: "Escape" });
  await waitFor(() => expect(screen.getByLabelText("Filter").getAttribute("aria-pressed")).toBe("true"));
  expect(contents(container)).toEqual(["Urgent chore"]);
  if (real && window.__TAURI__) window.__TAURI__.core.invoke = real;
});

it("an unsaved preview is dropped when the panel closes without a valid draft", async () => {
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  const before = contents(container);
  fireEvent.click(screen.getByLabelText("Filter"));
  fireEvent.change(await screen.findByLabelText("Query"), { target: { value: "p1 &" } });
  expect(contents(container)).toEqual(before);
  fireEvent.keyDown(document, { key: "Escape" });
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(contents(container)).toEqual(before);
  expect(screen.getByLabelText("Filter").getAttribute("aria-pressed")).toBe("false");
});

const follows = (a: Element | null, b: Element | null) =>
  a !== null && b !== null && Boolean(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);

it("the chips sit under the subtitle", async () => {
  await arrangeView({ kind: "today" }, arrangement({ filter: "p1" }));
  const { container } = await show({ kind: "today" });
  expect(container.querySelector(".view-sub")).not.toBeNull();
  expect(follows(container.querySelector(".view-sub"), container.querySelector(".arrange-bar"))).toBe(true);
});

it("on a saved filter's page the chips sit under its query line", async () => {
  await arrangeView(
    { kind: "filter", id: "f-p1" },
    arrangement({ order: [{ field: "priority", reversed: false }] }),
  );
  const { container } = await show({ kind: "filter", id: "f-p1", name: "Important", query: "p1" });
  expect(container.querySelector(".view-sub")?.textContent).toBe("p1");
  expect(follows(container.querySelector(".view-sub"), container.querySelector(".arrange-bar"))).toBe(true);
});

it("a filtered section's delete warning counts everything the section holds", async () => {
  await dispatchAction({ type: "task_update", id: "t-laundry", priority: "None" } as never);
  await addTask("Urgent chore", { section_id: "s-chores", priority: "P1" });
  await arrangeView(homePlace, arrangement({ filter: "p1" }));
  const { container } = await show({ kind: "project", id: "p-home", name: "Home" });
  const block = container.querySelector(".section-block") as HTMLElement;
  expect(block.querySelector(".section-title .muted")?.textContent).toBe("1");
  fireEvent.click(screen.getByLabelText("Section actions"));
  fireEvent.click(within(popup() as HTMLElement).getByText("Delete"));
  expect(popup()?.querySelector(".confirm-text")?.textContent).toBe(
    "Delete section “Chores” and its 2 tasks?",
  );
});
