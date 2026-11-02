import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { App } from "../app/App";
import { UPCOMING_DAYS } from "../app/view";
import { localToday } from "../clock";
import { sidebarCounts } from "../core";
import { addLabel } from "../ipc/actions";
import { invoke } from "../ipc/invoke";
import { loadCoreForTests } from "../test/wasm";

vi.mock("../core", async (original) => {
  const actual = await original<typeof import("../core")>();
  return { ...actual, sidebarCounts: vi.fn(actual.sidebarCounts) };
});

beforeAll(() => {
  loadCoreForTests();
});

afterEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset["theme"];
});

const sidebar = () => document.querySelector(".sidebar") as HTMLElement;
const title = () => document.querySelector(".view-title")?.textContent;
const active = () => sidebar().querySelector(".nav-item.active")?.textContent?.trim();

async function start() {
  const r = render(<App />);
  await screen.findByText("Do laundry");
  return r;
}

it("the brand row has the logo first and the theme and settings buttons last", async () => {
  await start();
  const kids = [...(sidebar().querySelector(".brand") as HTMLElement).children].map((c) => c.className);
  expect(kids[0]).toContain("brand-mark");
  expect(kids.slice(-2).map((c) => c.split(" ")[1])).toEqual(["theme-toggle", "settings-btn"]);
  expect(sidebar().querySelector(".brand")?.textContent).toBe("Keel");
  expect(kids[1]).toContain("brand-name");
});

it("a search field sits under the brand row, and opens Search on a phone", async () => {
  await start();
  const field = sidebar().querySelector(".sidebar-search-input") as HTMLInputElement;
  const brand = sidebar().querySelector(".brand") as HTMLElement;
  expect(brand.nextElementSibling?.contains(field)).toBe(true);
  expect(field.readOnly).toBe(true);
  fireEvent.click(field);
  await waitFor(() => expect(title()).toBe("Search"));
  expect(await screen.findByPlaceholderText("Search tasks, descriptions and labels")).toBeTruthy();
  expect(within(sidebar()).queryByText("Search", { selector: ".nav-item" })).toBeNull();
});

it("on desktop the field types the search, and results fill the main area", async () => {
  vi.stubGlobal("matchMedia", (q: string) => ({
    matches: q.includes("min-width"),
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
  await start();
  const field = sidebar().querySelector(".sidebar-search-input") as HTMLInputElement;
  expect(field.readOnly).toBe(false);
  fireEvent.focus(field);
  await waitFor(() => expect(title()).toBe("Search"));
  expect(document.querySelector(".search-input")).toBeNull();
  fireEvent.change(field, { target: { value: "laundry" } });
  await waitFor(() =>
    expect(document.querySelector(".list-body > .view-sub")?.textContent).toBe("2 results for laundry"),
  );
  expect(document.querySelector("main.main .task-card .task-row")).not.toBeNull();
  vi.unstubAllGlobals();
});

it("/ focuses the search on desktop", async () => {
  vi.stubGlobal("matchMedia", (q: string) => ({
    matches: q.includes("min-width"),
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
  await start();
  fireEvent.keyDown(document, { key: "/" });
  await waitFor(() => expect(title()).toBe("Search"));
  await waitFor(() => expect(document.activeElement?.className).toBe("sidebar-search-input"));
  vi.unstubAllGlobals();
});

it("the fixed views, projects, labels and filters each sit in a card", async () => {
  await start();
  const cards = [...sidebar().querySelectorAll(".nav-card")];
  expect(cards.length).toBeGreaterThanOrEqual(4);
  expect(cards[0]?.textContent).toContain("Inbox");
  expect(cards[0]?.textContent).toContain("Upcoming");
  expect([...sidebar().querySelectorAll(".section-head")].map((h) => h.textContent)).toEqual([
    "My Projects",
    "Labels",
    "Filters",
  ]);
});

it("rows show how many tasks each view lists", async () => {
  await start();
  const count = (name: string) =>
    within(sidebar()).getByText(name).closest(".nav-item")?.querySelector(".count")?.textContent;
  // From the mock's tasks, through keel-core's rule (its own tests cover the rule).
  const all = await invoke("all_tasks", {});
  const counts = sidebarCounts(all, localToday(), UPCOMING_DAYS);
  const ofProject = (id: string) => String(counts.projects.find((c) => c.project_id === id)?.count ?? "");
  expect(count("Today")).toBe(String(counts.today));
  expect(count("Upcoming")).toBe(String(counts.upcoming));
  expect(count("Inbox")).toBe(ofProject("p-inbox"));
  expect(count("Home")).toBe(ofProject("p-home"));
  expect(count("Home")).not.toBe("");
});

it("typing a desktop search doesn't count the sidebar's tasks again", async () => {
  vi.stubGlobal("matchMedia", (q: string) => ({
    matches: q.includes("min-width"),
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
  await start();
  const field = sidebar().querySelector(".sidebar-search-input") as HTMLInputElement;
  fireEvent.focus(field);
  await waitFor(() => expect(title()).toBe("Search"));
  await act(async () => {});
  const before = vi.mocked(sidebarCounts).mock.calls.length;
  fireEvent.change(field, { target: { value: "l" } });
  fireEvent.change(field, { target: { value: "la" } });
  expect(vi.mocked(sidebarCounts).mock.calls.length).toBe(before);
  vi.unstubAllGlobals();
});

it("a project's icon is a bold Phosphor icon, not a typed #", async () => {
  await start();
  const icon = within(sidebar()).getByText("Home").closest(".nav-item")?.querySelector(".nav-icon");
  expect(icon?.textContent).toBe("");
  expect(icon?.querySelector("svg")).not.toBeNull();
});

it("labels show tags in their color", async () => {
  await start();
  const row = within(sidebar()).getByText("errand").closest(".nav-item") as HTMLElement;
  const tag = row.querySelector<HTMLElement>(".nav-icon.label [style]");
  expect(tag?.style.color).toMatch(/^var\(--label-/);
  expect(tag?.querySelector("svg")).not.toBeNull();
});

it("a label with no color has a plain tag, like the other nav icons", async () => {
  await addLabel("plain");
  await start();
  const row = within(sidebar()).getByText("plain").closest(".nav-item") as HTMLElement;
  const tag = row.querySelector<HTMLElement>(".nav-icon.label .nav-tag");
  expect(tag?.style.color).toBe("");
});

it("nav rows switch the view and mark it active", async () => {
  await start();
  fireEvent.click(within(sidebar()).getByText("Upcoming"));
  await waitFor(() => expect(title()).toBe("Upcoming"));
  expect(active()).toContain("Upcoming");
  fireEvent.click(within(sidebar()).getByText("Home"));
  await waitFor(() => expect(title()).toBe("Home"));
  expect(active()).toContain("Home");
});

it("adding a project: + focuses the field, Enter adds it, + again closes", async () => {
  await start();
  fireEvent.click(screen.getByTitle("Add project"));
  const input = screen.getByPlaceholderText("Project name, then Enter");
  expect(document.activeElement).toBe(input);
  fireEvent.change(input, { target: { value: "Garden" } });
  fireEvent.submit(input);
  await waitFor(() => expect(within(sidebar()).getByText("Garden")).toBeTruthy());
  expect(screen.queryByPlaceholderText("Project name, then Enter")).toBeNull();
  fireEvent.click(screen.getByTitle("Add label"));
  const label = screen.getByPlaceholderText("Label name, then Enter");
  expect(document.activeElement).toBe(label);
  fireEvent.click(screen.getByTitle("Add label"));
  expect(screen.queryByPlaceholderText("Label name, then Enter")).toBeNull();
});

it("renaming a project keeps its open view's title in sync", async () => {
  await start();
  fireEvent.click(within(sidebar()).getByText("Home"));
  await waitFor(() => expect(title()).toBe("Home"));
  const row = within(sidebar()).getByText("Home").closest(".nav-item") as HTMLElement;
  fireEvent.click(within(row).getByTitle("Rename"));
  const input = sidebar().querySelector<HTMLInputElement>(".project-form input");
  expect(document.activeElement).toBe(input);
  expect(input?.value).toBe("Home");
  fireEvent.change(input as HTMLInputElement, { target: { value: "House" } });
  fireEvent.submit(input as HTMLInputElement);
  await waitFor(() => expect(title()).toBe("House"));
  expect(within(sidebar()).getByText("House")).toBeTruthy();
});

it("deleting a project asks first, then goes to the Inbox", async () => {
  await start();
  fireEvent.click(within(sidebar()).getByText("Home"));
  await waitFor(() => expect(title()).toBe("Home"));
  const row = within(sidebar()).getByText("Home").closest(".nav-item") as HTMLElement;
  fireEvent.click(within(row).getByTitle("Delete"));
  expect(sidebar().querySelector(".confirm-text")?.textContent).toBe("Delete “Home” and all its tasks?");
  fireEvent.click(within(sidebar()).getByText("Delete"));
  await waitFor(() => expect(title()).toBe("Inbox"));
  expect(within(sidebar()).queryByText("Home")).toBeNull();
});

it("a label's delete warning and rename", async () => {
  await start();
  const row = within(sidebar()).getByText("errand").closest(".nav-item") as HTMLElement;
  fireEvent.click(within(row).getByTitle("Delete"));
  expect(sidebar().querySelector(".confirm-text")?.textContent).toBe(
    "Delete label “errand”? It’s removed from its tasks.",
  );
  fireEvent.click(within(sidebar()).getByText("Cancel"));
  const again = within(sidebar()).getByText("errand").closest(".nav-item") as HTMLElement;
  fireEvent.click(within(again).getByTitle("Rename"));
  const input = sidebar().querySelector<HTMLInputElement>(".project-form input") as HTMLInputElement;
  fireEvent.change(input, { target: { value: "chores" } });
  fireEvent.submit(input);
  await waitFor(() => expect(within(sidebar()).getByText("chores")).toBeTruthy());
});

it("a new filter: Save waits for a valid query, then opens the filter", async () => {
  await start();
  fireEvent.click(screen.getByTitle("Add filter"));
  const name = screen.getByPlaceholderText("Filter name");
  expect(document.activeElement).toBe(name);
  const query = screen.getByPlaceholderText("Query, e.g. today & p1");
  const save = screen.getByText("Add filter") as HTMLButtonElement;
  expect(save.disabled).toBe(true);
  fireEvent.change(name, { target: { value: "Urgent" } });
  fireEvent.change(query, { target: { value: "p1 &" } });
  await waitFor(() =>
    expect(sidebar().querySelector(".filter-problem")?.textContent).toBe("expected a term after '&'"),
  );
  expect(save.disabled).toBe(true);
  fireEvent.change(query, { target: { value: "p1" } });
  await waitFor(() => expect(save.disabled).toBe(false));
  expect(sidebar().querySelector(".filter-hint")).not.toBeNull();
  fireEvent.submit(query);
  await waitFor(() => expect(title()).toBe("Urgent"));
  expect(document.querySelector(".view-sub")?.textContent).toBe("p1");
});

it("a stale filter check never overrides a newer one", async () => {
  const resolvers: ((v: unknown) => void)[] = [];
  const rejecters: ((e: unknown) => void)[] = [];
  const real = (await import("../ipc/mock")).mockHandlers;
  window.__TAURI__ = {
    core: {
      invoke: (cmd, args) => {
        if (cmd === "check_filter") {
          return new Promise((res, rej) => {
            resolvers.push(res);
            rejecters.push(rej);
          });
        }
        const a = (args as { args: unknown }).args;
        return (
          (real as unknown as Record<string, (x: unknown) => Promise<unknown>>)[cmd]?.(a) ??
          Promise.resolve(null)
        );
      },
    },
  };
  try {
    await start();
    fireEvent.click(screen.getByTitle("Add filter"));
    const query = screen.getByPlaceholderText("Query, e.g. today & p1");
    fireEvent.change(query, { target: { value: "p1 &" } });
    fireEvent.change(query, { target: { value: "p1" } });
    await act(async () => resolvers[1]?.(null));
    await act(async () => rejecters[0]?.("expected a term after '&'"));
    expect(sidebar().querySelector(".filter-problem")).toBeNull();
  } finally {
    delete window.__TAURI__;
  }
});

it("editing and deleting a filter", async () => {
  await start();
  const row = within(sidebar()).getByText("Important").closest(".nav-item") as HTMLElement;
  fireEvent.click(within(row).getByTitle("Edit filter"));
  const name = screen.getByPlaceholderText("Filter name") as HTMLInputElement;
  expect(name.value).toBe("Important");
  fireEvent.change(name, { target: { value: "Top" } });
  await waitFor(() => expect((screen.getByText("Save") as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(screen.getByText("Save"));
  await waitFor(() => expect(within(sidebar()).getByText("Top")).toBeTruthy());
  const top = within(sidebar()).getByText("Top").closest(".nav-item") as HTMLElement;
  fireEvent.click(within(top).getByTitle("Delete filter"));
  expect(sidebar().querySelector(".confirm-text")?.textContent).toBe(
    "Delete filter “Top”? Your tasks aren’t affected.",
  );
  fireEvent.click(within(sidebar()).getByText("Delete"));
  await waitFor(() => expect(within(sidebar()).queryByText("Top")).toBeNull());
  expect((await invoke("list_filters", {})).map((f) => f.name)).not.toContain("Top");
});

it("the theme button cycles and sets data-theme", async () => {
  await start();
  const button = sidebar().querySelector(".theme-toggle") as HTMLButtonElement;
  expect(button.title).toBe("Theme: System (click for Light)");
  fireEvent.click(button);
  expect(document.documentElement.dataset["theme"]).toBe("light");
  expect(button.title).toBe("Theme: Light (click for Dark)");
  fireEvent.click(button);
  expect(document.documentElement.dataset["theme"]).toBe("dark");
  fireEvent.click(button);
  expect(document.documentElement.dataset["theme"]).toBeUndefined();
});

it("quick theme clicks each step from the latest theme", async () => {
  await start();
  const button = sidebar().querySelector(".theme-toggle") as HTMLButtonElement;
  act(() => {
    button.click();
    button.click();
  });
  expect(document.documentElement.dataset["theme"]).toBe("dark");
});

it("picking a view in the drawer closes it once the view has loaded", async () => {
  const { container } = await start();
  fireEvent.click(screen.getByTitle("Open menu"));
  expect(container.querySelector(".app")?.className).toBe("app drawer-open");
  fireEvent.click(within(sidebar()).getByText("Inbox"));
  await waitFor(() => expect(title()).toBe("Inbox"));
  await waitFor(() => expect(container.querySelector(".app")?.className).toBe("app"));
});

it("the gear opens Settings and × closes it", async () => {
  await start();
  expect(document.querySelector(".modal")).toBeNull();
  fireEvent.click(sidebar().querySelector(".settings-btn") as HTMLElement);
  await screen.findByText("Task reminders");
  expect(document.querySelector(".modal-title")?.textContent).toBe("Settings");
  fireEvent.click(within(document.querySelector(".modal-head") as HTMLElement).getByTitle("Close"));
  expect(document.querySelector(".modal")).toBeNull();
});

it("a mark picked in Settings is the one rows complete with", async () => {
  await start();
  fireEvent.click(sidebar().querySelector(".settings-btn") as HTMLElement);
  await screen.findByText("Task reminders");
  fireEvent.click(screen.getByTitle("Pine"));
  fireEvent.click(within(document.querySelector(".modal-head") as HTMLElement).getByTitle("Close"));
  const row = screen.getByText("Do laundry").closest(".task-row") as HTMLElement;
  // The pine's viewBox, as DoneMark draws it in the circle.
  expect(row.querySelector(".check svg.done-mark")?.getAttribute("viewBox")).toBe("20 12 24 22");
});

it.each([
  ["Add project", "Project name, then Enter"],
  ["Add label", "Label name, then Enter"],
  ["Add filter", "Filter name"],
])("%s closes when you click outside it", async (add, placeholder) => {
  await start();
  fireEvent.click(screen.getByTitle(add));
  const field = screen.getByPlaceholderText(placeholder);
  fireEvent.click(field);
  expect(screen.getByPlaceholderText(placeholder)).toBeTruthy();
  fireEvent.click(document.body);
  expect(screen.queryByPlaceholderText(placeholder)).toBeNull();
});

it("a click on a row closes the add form and still opens the row", async () => {
  await start();
  fireEvent.click(screen.getByTitle("Add project"));
  fireEvent.click(within(sidebar()).getByText("Home"));
  expect(screen.queryByPlaceholderText("Project name, then Enter")).toBeNull();
  await waitFor(() => expect(title()).toBe("Home"));
});

it("another section's plus swaps the open add form", async () => {
  await start();
  fireEvent.click(screen.getByTitle("Add project"));
  fireEvent.click(screen.getByTitle("Add label"));
  expect(screen.queryByPlaceholderText("Project name, then Enter")).toBeNull();
  expect(document.activeElement).toBe(screen.getByPlaceholderText("Label name, then Enter"));
});

it("a drag that starts in the field and ends outside keeps the form open", async () => {
  await start();
  fireEvent.click(screen.getByTitle("Add project"));
  fireEvent.pointerDown(screen.getByPlaceholderText("Project name, then Enter"));
  fireEvent.click(document.body);
  expect(screen.getByPlaceholderText("Project name, then Enter")).toBeTruthy();
});

it("renaming closes unchanged when you click outside it", async () => {
  await start();
  const row = within(sidebar()).getByText("Home").closest(".nav-item") as HTMLElement;
  fireEvent.click(within(row).getByTitle("Rename"));
  const input = sidebar().querySelector<HTMLInputElement>(".project-form input") as HTMLInputElement;
  fireEvent.change(input, { target: { value: "House" } });
  fireEvent.click(document.body);
  expect(sidebar().querySelector(".project-form")).toBeNull();
  expect(within(sidebar()).getByText("Home")).toBeTruthy();
});

it("editing a filter closes when you click outside it", async () => {
  await start();
  const row = within(sidebar()).getByText("Important").closest(".nav-item") as HTMLElement;
  fireEvent.click(within(row).getByTitle("Edit filter"));
  expect(sidebar().querySelector(".filter-form")).toBeTruthy();
  fireEvent.click(document.body);
  expect(sidebar().querySelector(".filter-form")).toBeNull();
});
