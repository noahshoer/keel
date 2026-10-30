import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, expect, it } from "vitest";
import { App } from "../app/App";
import { invoke } from "../ipc/invoke";
import { loadCoreForTests } from "../test/wasm";

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

it("nav rows switch the view and mark it active", async () => {
  await start();
  fireEvent.click(within(sidebar()).getByText("Upcoming"));
  await waitFor(() => expect(title()).toBe("Upcoming"));
  expect(active()).toBe("Upcoming");
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
