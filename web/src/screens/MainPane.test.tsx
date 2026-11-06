import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useState } from "react";
import { beforeAll, expect, it, vi } from "vitest";
import { useLoaded } from "../app/load";
import type { View } from "../app/view";
import { localToday } from "../clock";
import { ROW_PRESS } from "../gestures/long-press";
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
