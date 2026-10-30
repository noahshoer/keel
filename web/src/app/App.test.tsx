import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, expect, it } from "vitest";
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
  expect(container.querySelector(".sidebar .nav-item.active .count")?.textContent).toBe("3");
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
