import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { mockPageScrollHeld } from "../ipc/mock";
import { setWidth, touch } from "../test/touch";
import { loadCoreForTests } from "../test/wasm";
import { App } from "./App";

beforeAll(() => {
  loadCoreForTests();
});

it("renders the sidebar and Today in the main pane", async () => {
  const { container } = render(<App />);
  expect(await screen.findByText("Do laundry")).toBeTruthy();
  expect(container.querySelector(".app > .sidebar")).not.toBeNull();
  expect(container.querySelector(".app > main.main > .content .view-title")?.textContent).toBe("Today");
  // The sidebar lists the user's projects (not the inbox) and Today's count.
  expect(container.querySelector(".sidebar")?.textContent).toContain("Home");
  // The count comes from every task's date (the mock's Today list is hand-picked).
  expect(Number(container.querySelector(".sidebar .nav-item.active .count")?.textContent)).toBeGreaterThan(0);
});

it("the menu button opens the drawer and the scrim closes it", async () => {
  const { container } = render(<App />);
  await screen.findByText("Do laundry");
  fireEvent.click(screen.getByTitle("Open menu"));
  expect(container.querySelector(".app")?.className).toBe("app drawer-open");
  fireEvent.click(container.querySelector(".drawer-scrim") as HTMLElement);
  expect(container.querySelector(".app")?.className).toBe("app");
});

it("a failed load shows the error", async () => {
  window.__TAURI__ = { core: { invoke: () => Promise.reject("database is locked") } };
  try {
    const { container } = render(<App />);
    await waitFor(() =>
      expect(container.querySelector(".error")?.textContent).toBe("Error: database is locked"),
    );
  } finally {
    delete window.__TAURI__;
  }
  await act(async () => {});
});

const appClass = (c: HTMLElement) => c.querySelector(".app")?.className;
const rowOf = (c: HTMLElement, id: string) =>
  c.querySelector(`.main .task[data-menu-id="${id}"]`) as HTMLElement;

it("a row's body opens the task view; the scrim closes it at desktop width", async () => {
  setWidth(1024);
  const { container } = render(<App />);
  await screen.findByText("Do laundry");
  expect(container.querySelector(".tv-sheet")).toBeNull();
  fireEvent.click(rowOf(container, "t-laundry").querySelector(".task-body") as HTMLElement);
  const sheet = container.querySelector(".app > .tv-sheet");
  expect(sheet?.querySelector(".tv-grabber")).not.toBeNull();
  const view = sheet?.querySelector('.tv-stack > .task-view[data-view-id="t-laundry"]');
  expect(view?.querySelector<HTMLTextAreaElement>(".tv-title textarea")?.value).toBe("Do laundry");
  expect(appClass(container)).toBe("app view-open");
  fireEvent.click(container.querySelector(".tv-scrim") as HTMLElement);
  await waitFor(() => expect(container.querySelector(".tv-sheet")).toBeNull());
  expect(container.querySelector(".tv-scrim")).toBeNull();
  expect(appClass(container)).toBe("app");
});

it("the page under the task view holds still while it's open", async () => {
  // iOS: WebKit scrolls the page to reveal a field focused in the sheet;
  // Swift holds it (keel-ios PageScroll) while the view is open.
  setWidth(1024);
  const { container } = render(<App />);
  await screen.findByText("Do laundry");
  expect(mockPageScrollHeld()).toBe(false);
  fireEvent.click(rowOf(container, "t-laundry").querySelector(".task-body") as HTMLElement);
  await waitFor(() => expect(mockPageScrollHeld()).toBe(true));
  fireEvent.click(container.querySelector(".tv-scrim") as HTMLElement);
  await waitFor(() => expect(mockPageScrollHeld()).toBe(false));
});

it("deleting the open task from its row closes the sheet after the reload", async () => {
  setWidth(1024);
  const { container } = render(<App />);
  await screen.findByText("Do laundry");
  fireEvent.click(rowOf(container, "t-laundry").querySelector(".task-body") as HTMLElement);
  expect(container.querySelector(".tv-sheet")).not.toBeNull();
  fireEvent.click(rowOf(container, "t-laundry").querySelector('[title="Delete task"]') as HTMLElement);
  await waitFor(() => expect(container.querySelector(".tv-sheet")).toBeNull());
  expect(appClass(container)).toBe("app");
});

it("deleting a sub-task from its ••• makes the parent active in the same sheet", async () => {
  setWidth(1024);
  const { container } = render(<App />);
  await screen.findByText("Do laundry");
  fireEvent.click(rowOf(container, "t-laundry").querySelector(".task-body") as HTMLElement);
  const sheet = container.querySelector(".tv-sheet");
  const parentView = sheet?.querySelector('.task-view[data-view-id="t-laundry"]');
  fireEvent.click(parentView?.querySelector(".tv-sub .tv-sub-body") as HTMLElement);
  const child = () => container.querySelector('.task-view[data-view-id="t-sheets"]');
  expect(child()?.className).toContain("task-view");
  // Every class the app takes from here on: the view stays open throughout.
  const app = container.querySelector(".app") as HTMLElement;
  const classes: string[] = [];
  const watch = new MutationObserver(() => classes.push(app.className));
  watch.observe(app, { attributes: true, attributeFilter: ["class"] });
  fireEvent.click(child()?.querySelector(".tv-more") as HTMLElement);
  fireEvent.click(within(child() as HTMLElement).getByText("Delete task"));
  await waitFor(() => expect(child()).toBeNull());
  watch.disconnect();
  expect(container.querySelector(".tv-sheet")).toBe(sheet);
  expect(container.querySelector(".tv-stack > .task-view")).toBe(parentView);
  expect(container.querySelector(".tv-under")).toBeNull();
  expect(classes.every((c) => c.includes("view-open"))).toBe(true);
});

it("on a phone, a swipe right opens the drawer and a swipe left closes it", async () => {
  setWidth(390);
  try {
    const { container } = render(<App />);
    await screen.findByText("Do laundry");
    const main = container.querySelector(".main") as HTMLElement;
    act(() => {
      touch("touchstart", main, 40, 300, 0);
      touch("touchend", main, 160, 310, 200);
    });
    expect(appClass(container)).toBe("app drawer-open");
    act(() => {
      touch("touchstart", main, 300, 300, 1000);
      touch("touchend", main, 180, 305, 1200);
    });
    expect(appClass(container)).toBe("app");
  } finally {
    setWidth(1024);
  }
});

it("the page holds still while the drawer is open", async () => {
  // WebKit would otherwise shift the whole page, drawer and all, to reveal a focused field.
  render(<App />);
  fireEvent.click(await screen.findByTitle("Open menu"));
  await waitFor(() => expect(mockPageScrollHeld()).toBe(true));
  fireEvent.click(document.querySelector(".drawer-scrim") as Element);
  await waitFor(() => expect(mockPageScrollHeld()).toBe(false));
});

describe("the ••• on a project, label or filter page", () => {
  const open = async (name: string) => {
    const r = render(<App />);
    await waitFor(() => expect(r.container.querySelector(".view-title")).not.toBeNull());
    fireEvent.click(within(r.container.querySelector(".sidebar") as HTMLElement).getByText(name));
    await waitFor(() => expect(r.container.querySelector(".view-title")?.textContent).toBe(name));
    return r;
  };
  const more = (c: HTMLElement) => c.querySelector(".view-head .view-more") as HTMLElement;
  const popup = () => document.querySelector(".row-menu") as HTMLElement;
  const pick = (c: HTMLElement, item: string) => {
    fireEvent.click(more(c));
    fireEvent.click(within(popup()).getByText(item));
  };

  it.each(["Inbox", "Today", "Upcoming"])("%s has none", async (name) => {
    const { container } = await open(name);
    expect(more(container)).toBeNull();
  });

  it("renames a project from its page, keeping the page's title in sync", async () => {
    const { container } = await open("Home");
    pick(container, "Edit");
    const field = popup().querySelector("input") as HTMLInputElement;
    expect(document.activeElement).toBe(field);
    expect(field.value).toBe("Home");
    fireEvent.change(field, { target: { value: "House" } });
    fireEvent.submit(field);
    await waitFor(() => expect(container.querySelector(".view-title")?.textContent).toBe("House"));
    expect(popup()).toBeNull();
  });

  it("renames a label from its page", async () => {
    const { container } = await open("errand");
    pick(container, "Edit");
    const field = popup().querySelector("input") as HTMLInputElement;
    fireEvent.change(field, { target: { value: "chores" } });
    fireEvent.submit(field);
    await waitFor(() => expect(container.querySelector(".view-title")?.textContent).toBe("chores"));
  });

  it("edits a filter's name and query from its page", async () => {
    const { container } = await open("Important");
    pick(container, "Edit");
    const name = within(popup()).getByPlaceholderText("Filter name") as HTMLInputElement;
    expect(document.activeElement).toBe(name);
    fireEvent.change(name, { target: { value: "Top" } });
    const save = within(popup()).getByText("Save") as HTMLButtonElement;
    await waitFor(() => expect(save.disabled).toBe(false));
    fireEvent.click(save);
    await waitFor(() => expect(container.querySelector(".view-title")?.textContent).toBe("Top"));
  });

  it("deletes after asking, then goes to the Inbox", async () => {
    const { container } = await open("Home");
    pick(container, "Delete");
    expect(popup().querySelector(".confirm-text")?.textContent).toBe("Delete “Home” and all its tasks?");
    fireEvent.click(within(popup()).getByText("Delete"));
    await waitFor(() => expect(container.querySelector(".view-title")?.textContent).toBe("Inbox"));
  });

  it("••• closes it again, and so does a click outside", async () => {
    const { container } = await open("Home");
    fireEvent.click(more(container));
    expect(popup()).not.toBeNull();
    fireEvent.click(more(container));
    expect(popup()).toBeNull();
    fireEvent.click(more(container));
    fireEvent.click(document.body);
    expect(popup()).toBeNull();
  });

  it("typing in the edit form doesn't close it", async () => {
    const { container } = await open("Home");
    pick(container, "Edit");
    const field = popup().querySelector("input") as HTMLInputElement;
    fireEvent.scroll(field);
    fireEvent.touchStart(field);
    expect(popup()).not.toBeNull();
  });
});
