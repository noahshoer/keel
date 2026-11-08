import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, expect, it } from "vitest";
import { App } from "../app/App";
import { loadCoreForTests } from "../test/wasm";

beforeAll(() => {
  loadCoreForTests();
});

async function openSearch() {
  const r = render(<App />);
  await screen.findByText("Do laundry");
  const nav = r.container.querySelector(".sidebar") as HTMLElement;
  fireEvent.click(document.querySelector(".search-btn") as HTMLElement);
  const input = await screen.findByPlaceholderText("Search tasks, descriptions and labels");
  return { ...r, nav, input: input as HTMLInputElement };
}

it("an empty query invites a search, with this platform's hint", async () => {
  const { container } = await openSearch();
  expect(container.querySelector(".empty .big")?.textContent).toBe("Search your tasks");
  expect(container.querySelector(".empty div:last-child")?.textContent).toContain("Every word must match");
});

it("typing shows the open results with their project", async () => {
  const { container, input } = await openSearch();
  fireEvent.change(input, { target: { value: "laundry" } });
  await waitFor(() =>
    expect(container.querySelector(".list-body > .view-sub")?.textContent).toBe("2 results"),
  );
  // Scoped to the live pane: the push leaves a still copy of Today (.view-ghost) for a moment.
  expect([...container.querySelectorAll("main.main .task-content")].map((e) => e.textContent)).toEqual([
    "Do laundry",
    "Wash the sheets",
  ]);
  expect(container.querySelector("main.main .task-project")?.textContent).toBe("Home / Chores");
  // As in the mockup, a result with sub-tasks shows their count.
  expect(container.querySelector("main.main .task-count")?.textContent).toBe("0 of 1");
});

it("counts the results above one card of the list's own rows", async () => {
  const { container, input } = await openSearch();
  fireEvent.change(input, { target: { value: "laundry" } });
  await waitFor(() =>
    expect(container.querySelector(".list-body > .view-sub")?.textContent).toBe("2 results"),
  );
  const card = container.querySelector("main.main .task-card") as HTMLElement;
  expect(card.querySelectorAll(".task-row")).toHaveLength(2);
});

it("shows Inbox on a result from the Inbox", async () => {
  const { container, input } = await openSearch();
  fireEvent.change(input, { target: { value: "dentist" } });
  await waitFor(() => expect(container.querySelector("main.main .task-project")?.textContent).toBe("Inbox"));
});

it("Return closes the keyboard, keeping the results", async () => {
  const { input } = await openSearch();
  fireEvent.change(input, { target: { value: "laundry" } });
  input.focus();
  fireEvent.keyDown(input, { key: "Enter" });
  expect(document.activeElement).not.toBe(input);
  expect(input.value).toBe("laundry");
});

it("scrolling the results closes the keyboard", async () => {
  const { container, input } = await openSearch();
  input.focus();
  fireEvent.touchMove(container.querySelector("main.main") as HTMLElement);
  expect(document.activeElement).not.toBe(input);
});

it("the magnifying glass on the main page opens Search", async () => {
  const r = render(<App />);
  await screen.findByText("Do laundry");
  const main = r.container.querySelector("main.main") as HTMLElement;
  fireEvent.click(within(main).getByRole("button", { name: "Search" }));
  expect(await screen.findByPlaceholderText("Search tasks, descriptions and labels")).toBeTruthy();
  const live = r.container.querySelector("main.main") as HTMLElement;
  expect(within(live).queryByRole("button", { name: "Search" })).toBeNull();
});

it("on a phone Back leaves Search for the view before it", async () => {
  const { container } = await openSearch();
  fireEvent.click(
    within(container.querySelector("main.main") as HTMLElement).getByRole("button", { name: "Back" }),
  );
  await waitFor(() => expect(container.querySelector("main.main .view-title")?.textContent).toBe("Today"));
});

it("no open match but completed ones: points at Show completed", async () => {
  const { container, input } = await openSearch();
  fireEvent.change(input, { target: { value: "bins" } });
  await waitFor(() =>
    expect(container.querySelector(".empty .big")?.textContent).toBe("No tasks match “bins”"),
  );
  expect(container.querySelector(".empty div:last-child")?.textContent).toBe(
    "1 completed task(s) match — turn on Show completed to see them.",
  );
  fireEvent.click(container.querySelector("main.main .view-toggle") as HTMLElement);
  await waitFor(() => expect(container.querySelector(".task.completed")).not.toBeNull());
  fireEvent.change(input, { target: { value: "nothing" } });
  await waitFor(() =>
    expect(container.querySelector(".empty div:last-child")?.textContent).toBe("Try fewer or shorter words."),
  );
});

it("Escape and the clear button empty the box; the clear button keeps focus", async () => {
  const { container, input } = await openSearch();
  fireEvent.change(input, { target: { value: "laundry" } });
  fireEvent.keyDown(input, { key: "Escape" });
  expect(input.value).toBe("");
  fireEvent.change(input, { target: { value: "laundry" } });
  const clear = screen.getByLabelText("Clear search");
  input.focus();
  const down = fireEvent.mouseDown(clear);
  expect(down).toBe(false); // default prevented: focus stays in the field
  fireEvent.click(clear);
  expect(input.value).toBe("");
  expect(container.querySelector(".search-clear")).toBeNull();
});

it("the query survives leaving Search and coming back", async () => {
  const { nav, input } = await openSearch();
  fireEvent.change(input, { target: { value: "laundry" } });
  fireEvent.click(within(nav).getByText("Today"));
  await waitFor(() => expect(document.querySelector(".view-title")?.textContent).toBe("Today"));
  fireEvent.click(document.querySelector(".search-btn") as HTMLElement);
  const again = (await screen.findByPlaceholderText(
    "Search tasks, descriptions and labels",
  )) as HTMLInputElement;
  expect(again.value).toBe("laundry");
});

it("Cmd+K opens Search from anywhere and focuses the box", async () => {
  render(<App />);
  await screen.findByText("Do laundry");
  fireEvent.keyDown(document, { key: "k", metaKey: true });
  const input = await screen.findByPlaceholderText("Search tasks, descriptions and labels");
  await waitFor(() => expect(document.activeElement).toBe(input));
});

it("a slow earlier search never replaces newer results", async () => {
  const real = (await import("../ipc/mock")).mockHandlers as unknown as Record<
    string,
    (a: unknown) => Promise<unknown>
  >;
  const pending: Record<string, () => void> = {};
  window.__TAURI__ = {
    core: {
      invoke: (cmd, args) => {
        const a = (args as { args: { query?: string } }).args;
        if (cmd === "search") {
          return new Promise((res) => {
            pending[a.query ?? ""] = () => res(real["search"]?.(a));
          });
        }
        return real[cmd]?.(a) ?? Promise.resolve(null);
      },
    },
  };
  try {
    const { container, input } = await openSearch();
    fireEvent.change(input, { target: { value: "bins" } });
    fireEvent.change(input, { target: { value: "laundry" } });
    await act(async () => pending["laundry"]?.());
    await waitFor(() =>
      expect(container.querySelector(".list-body > .view-sub")?.textContent).toBe("2 results"),
    );
    await act(async () => pending["bins"]?.());
    expect(container.querySelector(".list-body > .view-sub")?.textContent).toBe("2 results");
    // While a newer query loads, the last results stay up (no "Searching…" flash).
    fireEvent.change(input, { target: { value: "laundryx" } });
    expect(container.querySelector("main.main .list-body > .view-sub")?.textContent).toBe("2 results");
    expect(container.querySelectorAll("main.main .task-row")).toHaveLength(2);
  } finally {
    delete window.__TAURI__;
  }
});

it("from Today the search covers everything: no scope chip", async () => {
  const { container } = await openSearch();
  expect(container.querySelector(".scope-chip")).toBeNull();
});

it("opened from a project, the search stays inside it until the chip is removed", async () => {
  const r = render(<App />);
  await screen.findByText("Do laundry");
  const nav = r.container.querySelector(".sidebar") as HTMLElement;
  fireEvent.click(within(nav).getByText("Home"));
  await waitFor(() => expect(document.querySelector(".view-title")?.textContent).toBe("Home"));
  fireEvent.click(document.querySelector(".search-btn") as HTMLElement);
  const chip = await screen.findByRole("button", { name: /Searching in #Home/ });
  expect(chip.textContent).toBe("#Home");
  // Leaving and opening from Today searches everything again.
  fireEvent.click(chip);
  expect(r.container.querySelector(".scope-chip")).toBeNull();
});

it("a query that isn't a filter says it is searched as words", async () => {
  const { container, input } = await openSearch();
  fireEvent.change(input, { target: { value: "laundry &" } });
  await waitFor(() =>
    expect(container.querySelector(".search-problem")?.textContent).toContain("Searching for the words"),
  );
  fireEvent.change(input, { target: { value: "laundry" } });
  await waitFor(() => expect(container.querySelector(".search-problem")).toBeNull());
});
