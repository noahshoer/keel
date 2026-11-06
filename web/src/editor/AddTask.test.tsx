import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { blankValues } from "../core";
import * as fx from "../ipc/fixtures";
import { loadCoreForTests } from "../test/wasm";
import { AddTask } from "./AddTask";

beforeAll(loadCoreForTests);
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

function add(main: boolean) {
  return render(
    <div className="content">
      <AddTask
        main={main}
        defaults={blankValues("p-inbox", null, "", [])}
        projects={fx.projects}
        sections={fx.sections}
        labels={fx.labels}
        onChanged={() => {}}
      />
    </div>,
  );
}

const PLACEHOLDER = "Task name — try “Call mom tomorrow 5pm p1 #Home @phone”";

it("floats a round add button over the view's own list, and it opens the editor", () => {
  const { container } = add(true);
  const fab = screen.getByRole("button", { name: "New task" });
  expect(container.querySelector(".content > .fab")).toBe(fab);
  fireEvent.click(fab);
  expect(screen.getByPlaceholderText(PLACEHOLDER)).toBeTruthy();
  // The editor stands in for the button while it's open.
  expect(screen.queryByRole("button", { name: "New task" })).toBeNull();
});

it("has no floating button on a section's own row", () => {
  add(false);
  expect(screen.queryByRole("button", { name: "New task" })).toBeNull();
});

it("on touch, keeps the row's place in the list while the card is open, so the list doesn't move", () => {
  vi.stubGlobal("matchMedia", (q: string) => ({ matches: q.includes("coarse") }));
  const { container } = add(false);
  fireEvent.click(screen.getByText("Add task"));
  expect(screen.getByPlaceholderText(PLACEHOLDER)).toBeTruthy();
  // The card leaves for the keyboard (portaled to body); its row stays behind, inert.
  const row = container.querySelector(".add-task-row");
  expect(row).not.toBeNull();
  expect(row?.getAttribute("aria-hidden")).toBe("true");
  fireEvent.click(row as Element);
  expect(screen.getAllByPlaceholderText(PLACEHOLDER)).toHaveLength(1);
});

it("on a desktop, the editor takes the row's place", () => {
  const { container } = add(false);
  fireEvent.click(screen.getByText("Add task"));
  expect(container.querySelector(".add-task-row")).toBeNull();
});
