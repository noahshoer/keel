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
  fireEvent.click(within(nav).getByText("Search"));
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
  await waitFor(() => expect(container.querySelector(".content > .view-sub")?.textContent).toBe("2 tasks"));
  // Scoped to the live pane: the push leaves a still copy of Today (.view-ghost) for a moment.
  expect([...container.querySelectorAll("main.main .task-content")].map((e) => e.textContent)).toEqual([
    "Do laundry",
    "Wash the sheets",
  ]);
  expect(container.querySelector("main.main .task-project")?.textContent).toBe("Home / Chores");
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
  fireEvent.click(within(nav).getByText("Search"));
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
    await waitFor(() => expect(container.querySelector(".content > .view-sub")?.textContent).toBe("2 tasks"));
    await act(async () => pending["bins"]?.());
    expect(container.querySelector(".content > .view-sub")?.textContent).toBe("2 tasks");
  } finally {
    delete window.__TAURI__;
  }
});
