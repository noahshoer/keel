import { render, screen } from "@testing-library/react";
import { beforeAll, expect, test } from "vitest";
import { loadCoreForTests } from "../test/wasm";
import { Today } from "./Today";

beforeAll(() => {
  loadCoreForTests();
});

test("shows today's cards from the backend", async () => {
  const { container } = render(<Today />);
  // The mock backend's Today: "File the taxes" (overdue), "Do laundry" (P1,
  // due today) and its sub-task "Wash the sheets" (also due today).
  const parent = await screen.findByText("Do laundry");

  const root = container.firstElementChild;
  expect(root?.classList.contains("today")).toBe(true);

  // Grouped (as the Dioxus Today): the matched sub-task is not a card of its
  // own; it sits in its parent's card, and the parent counts it as 0/1.
  const cards = container.querySelectorAll(".task-card");
  expect(cards).toHaveLength(2);
  const card = parent.closest(".task-card");
  expect(card?.contains(parent)).toBe(true);
  expect(card?.contains(screen.getByText("Wash the sheets"))).toBe(true);

  const rows = container.querySelectorAll(".task-row");
  expect(rows).toHaveLength(3);
  const parentRow = parent.closest(".task-row");
  expect(parentRow?.querySelector(".task-count")?.textContent).toBe("0/1");

  // Due chips come from keel-core's labels.
  expect(parentRow?.querySelector(".due")?.textContent).toBe("Today");
  // Today's rows name where the task lives (Dioxus `show_project` for Today).
  expect(parentRow?.querySelector(".task-project")?.textContent).toBe("Home / Chores");
  expect(container.querySelector(".view-title")?.textContent).toBe("Today");
});

test("shows the empty state when nothing is due", async () => {
  const { container } = render(
    <Today load={async () => ({ matched: [], all: [], projects: [], sections: [] })} />,
  );
  expect(await screen.findByText("You're all done for today!")).toBeTruthy();
  expect(container.querySelectorAll(".task-card")).toHaveLength(0);
});
