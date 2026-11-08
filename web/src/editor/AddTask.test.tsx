import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { blankValues } from "../core";
import * as fx from "../ipc/fixtures";
import { invoke } from "../ipc/invoke";
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

const title = () => screen.getByPlaceholderText(PLACEHOLDER) as HTMLTextAreaElement;
const typeAndAdd = (text: string) => {
  fireEvent.change(title(), { target: { value: text } });
  fireEvent.click(screen.getByRole("button", { name: "Add task" }));
};
const added = async (content: string) => (await invoke("all_tasks", {})).find((t) => t.content === content);

it("new names ask first; Back returns to the card, and Save creates the ticked ones", async () => {
  add(false);
  fireEvent.click(screen.getByText("Add task"));
  typeAndAdd("Buy ink #wrok @errands");
  expect(screen.getByText("Create these with the task?")).toBeTruthy();
  expect(screen.getByText("Untick any you don’t want. They stay as text in the title.")).toBeTruthy();
  const project = screen.getByRole("checkbox", { name: /^New project/ });
  const label = screen.getByRole("checkbox", { name: /^New label/ });
  expect(project.getAttribute("aria-checked")).toBe("true");
  expect(label.getAttribute("aria-checked")).toBe("true");
  expect(project.textContent).toContain("1 of 1");
  expect(screen.getByRole("checkbox", { name: "#wrok" }).getAttribute("aria-checked")).toBe("true");

  fireEvent.click(screen.getByRole("button", { name: "Back to editing" }));
  expect(screen.queryByText("Create these with the task?")).toBeNull();
  expect(title().value).toBe("Buy ink #wrok @errands");
  expect(await added("Buy ink")).toBeUndefined();

  fireEvent.click(screen.getByRole("button", { name: "Add task" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "@errands" }));
  expect(screen.getByRole("checkbox", { name: /^New label/ }).getAttribute("aria-checked")).toBe("false");
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  expect(screen.queryByText("Create these with the task?")).toBeNull();
  await waitFor(async () => expect(await added("Buy ink @errands")).toBeDefined());
  const saved = await added("Buy ink @errands");
  expect(saved?.labels).toEqual([]);
  const wrok = (await invoke("list_projects", {})).find((p) => p.name === "wrok");
  expect(saved?.project_id).toBe(wrok?.id);
  expect((await invoke("list_labels", {})).some((l) => l.name === "errands")).toBe(false);
});

it("a title with no new names saves straight away", async () => {
  add(false);
  fireEvent.click(screen.getByText("Add task"));
  typeAndAdd("Call the bank @errand");
  expect(screen.queryByText("Create these with the task?")).toBeNull();
  await waitFor(async () => expect(await added("Call the bank")).toBeDefined());
});
