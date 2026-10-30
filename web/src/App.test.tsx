import { render, screen } from "@testing-library/react";
import { beforeAll, expect, test } from "vitest";
import { App } from "./App";
import { loadCoreForTests } from "./test/wasm";

beforeAll(() => {
  loadCoreForTests();
});

test("renders the shell with Today in the main pane", async () => {
  const { container } = render(<App />);
  expect(container.querySelector(".app > .sidebar")).not.toBeNull();
  expect(await screen.findByText("Do laundry")).toBeTruthy();
  expect(container.querySelector(".app > main.main > .content.today")).not.toBeNull();
  // The sidebar lists the user's projects (not the inbox) from the backend.
  expect(await screen.findByText("Home")).toBeTruthy();
});
