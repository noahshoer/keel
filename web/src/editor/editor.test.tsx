// The task editor, with the clock at Friday 2026-10-30 noon.
import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, expect, it, vi } from "vitest";
import type { EditorValues } from "../bindings";
import { blankValues, valuesFromTask } from "../core";
import * as fx from "../ipc/fixtures";
import { loadCoreForTests } from "../test/wasm";
import { TaskEditor } from "./TaskEditor";

beforeAll(() => {
  loadCoreForTests();
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(2026, 9, 30, 12));
});

afterAll(() => {
  vi.useRealTimers();
});

let onSubmit = vi.fn<(v: EditorValues) => void>();
let onCancel = vi.fn<() => void>();

beforeEach(() => {
  onSubmit = vi.fn();
  onCancel = vi.fn();
});

function adding() {
  render(
    <TaskEditor
      initial={blankValues("p-inbox", null, "", [])}
      projects={fx.projects}
      sections={fx.sections}
      labels={fx.labels}
      keepLabel={null}
      existing={null}
      submitText="Add task"
      quickAdd
      keepOpen
      onSubmit={onSubmit}
      onCancel={onCancel}
      onChanged={() => {}}
    />,
  );
  return screen.getByPlaceholderText(
    "Task name — try “Call mom tomorrow 5pm p1 #Home @phone”",
  ) as HTMLTextAreaElement;
}

const type = (title: HTMLTextAreaElement, text: string) =>
  fireEvent.change(title, { target: { value: text } });
const marks = () =>
  [...document.querySelectorAll(".title-mirror mark")].map((m) => `${m.className}:${m.textContent}`);
const backspaceAt = (title: HTMLTextAreaElement, at: number) => {
  title.setSelectionRange(at, at);
  return fireEvent.keyDown(title, { key: "Backspace" });
};

it("highlights what quick add understood and previews it", () => {
  const title = adding();
  type(title, "Call mom tomorrow p1");
  expect(marks()).toEqual(["hl hl-date:tomorrow", "hl hl-p1:p1"]);
  const chips = [...document.querySelectorAll(".quick-preview .quick-chip")].map(
    (c) => `${c.className}:${c.textContent}`,
  );
  expect(chips).toEqual(["quick-chip chip-date:Tomorrow", "quick-chip chip-priority:P1"]);
});

it("can't add a phrase on its own", () => {
  const title = adding();
  type(title, "tomorrow");
  expect((screen.getByText("Add task", { selector: "button" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.keyDown(title, { key: "Enter" });
  expect(onSubmit).not.toHaveBeenCalled();
});

it("Enter adds once, resets, and keeps the title focused", () => {
  const title = adding();
  title.focus();
  type(title, "Water plants");
  fireEvent.keyDown(title, { key: "Enter" });
  fireEvent.keyDown(title, { key: "Enter" });
  expect(onSubmit).toHaveBeenCalledTimes(1);
  expect(onSubmit.mock.calls[0]?.[0].content).toBe("Water plants");
  expect(title.value).toBe("");
  expect(document.activeElement).toBe(title);
});

it("Backspace right after a highlight turns it plain", () => {
  const title = adding();
  type(title, "Call mom tomorrow p1");
  expect(backspaceAt(title, "Call mom tomorrow".length)).toBe(false);
  expect(marks()).toEqual(["hl hl-p1:p1"]);
  expect(title.value).toBe("Call mom tomorrow p1");
  // Nothing ends there now, so the next Backspace deletes as usual.
  expect(backspaceAt(title, "Call mom tomorrow".length)).toBe(true);
});

it("un-highlights after non-ASCII text", () => {
  const title = adding();
  type(title, "Café tomorrow 🍰");
  expect(marks()).toEqual(["hl hl-date:tomorrow"]);
  expect(backspaceAt(title, "Café tomorrow".length)).toBe(false);
  expect(marks()).toEqual([]);
});

it("a picked date replaces the typed one", () => {
  const title = adding();
  type(title, "call mom tomorrow");
  fireEvent.click(screen.getByTitle("Date"));
  fireEvent.click(screen.getByText("Next week"));
  expect(title.value).toBe("call mom");
  expect(screen.getByTitle("Date").textContent).toBe("Monday");
  fireEvent.keyDown(title, { key: "Enter" });
  expect(onSubmit.mock.calls[0]?.[0]).toMatchObject({ content: "call mom", date: "2026-11-02" });
});

it("Escape closes the popover first, then cancels", () => {
  const title = adding();
  fireEvent.click(screen.getByTitle("Priority"));
  expect(document.querySelector(".pop-backdrop")).not.toBeNull();
  fireEvent.keyDown(title, { key: "Escape" });
  expect(document.querySelector(".popover")).toBeNull();
  expect(onCancel).not.toHaveBeenCalled();
  fireEvent.keyDown(title, { key: "Escape" });
  expect(onCancel).toHaveBeenCalledTimes(1);
});

it("picks a priority and a project from the menus", () => {
  const title = adding();
  type(title, "x");
  fireEvent.click(screen.getByTitle("Priority"));
  fireEvent.click(screen.getByText("Priority 1"));
  expect(screen.getByTitle("Priority").textContent).toBe("P1");
  fireEvent.click(screen.getByTitle("Project / section"));
  fireEvent.click(screen.getByText("Chores"));
  expect(screen.getByTitle("Project / section").textContent).toBe("Home / Chores");
  fireEvent.keyDown(title, { key: "Enter" });
  expect(onSubmit.mock.calls[0]?.[0]).toMatchObject({
    priority: "1",
    project_id: "p-home",
    section_id: "s-chores",
  });
});

it("edit mode never highlights and saves once", async () => {
  const task = fx.tasks.find((t) => t.id === "t-inbox");
  if (task === undefined) throw new Error("fixture");
  render(
    <TaskEditor
      initial={valuesFromTask({ ...task, content: "Call tomorrow p1" })}
      projects={fx.projects}
      sections={fx.sections}
      labels={fx.labels}
      keepLabel={null}
      existing={null}
      submitText="Save"
      quickAdd={false}
      keepOpen={false}
      onSubmit={onSubmit}
      onCancel={onCancel}
      onChanged={() => {}}
    />,
  );
  const title = screen.getByPlaceholderText("Task name") as HTMLTextAreaElement;
  expect(marks()).toEqual([]);
  expect(document.querySelector(".quick-preview")).toBeNull();
  await act(async () => {
    fireEvent.keyDown(title, { key: "Enter" });
    fireEvent.keyDown(title, { key: "Enter" });
  });
  expect(onSubmit).toHaveBeenCalledTimes(1);
});
