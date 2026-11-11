import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, expect, it, vi } from "vitest";
import type { Arrangement } from "../bindings";
import { sortChoices } from "../core";
import { loadCoreForTests } from "../test/wasm";
import { ArrangePanel } from "./ArrangePanel";
import { SortSheet } from "./SortSheet";

beforeAll(() => {
  loadCoreForTests();
});

const base: Arrangement = { filter: "", groups: [], order: [] };
const name = (field: string) => {
  const c = sortChoices().find((x) => x.field === field);
  if (!c) throw new Error(field);
  return c;
};
const sheet = (arrangement: Arrangement) => {
  const onChange = vi.fn();
  render(<SortSheet arrangement={arrangement} onChange={onChange} />);
  return onChange;
};

it("Add group lists only unused fields and picking one appends it", () => {
  const label = name("label");
  const onChange = sheet({ ...base, groups: ["project"] });
  fireEvent.click(screen.getByRole("button", { name: "Add group" }));
  expect(screen.queryByRole("button", { name: name("project").name })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: label.name }));
  expect(onChange).toHaveBeenCalledWith({ ...base, groups: ["project", "label"] });
});

it("Add order appends ascending", () => {
  const onChange = sheet(base);
  fireEvent.click(screen.getByRole("button", { name: "Add order" }));
  fireEvent.click(screen.getByRole("button", { name: name("due_date").name }));
  expect(onChange).toHaveBeenCalledWith({ ...base, order: [{ field: "due_date", reversed: false }] });
});

it("the direction button shows keel-core's label and flips", () => {
  const date = name("due_date");
  const onChange = sheet({ ...base, order: [{ field: "due_date", reversed: false }] });
  fireEvent.click(screen.getByRole("button", { name: date.order_label }));
  expect(onChange).toHaveBeenCalledWith({ ...base, order: [{ field: "due_date", reversed: true }] });
});

it("shows the reversed label when reversed", () => {
  sheet({ ...base, order: [{ field: "due_date", reversed: true }] });
  expect(screen.getByRole("button", { name: name("due_date").reversed_label })).toBeTruthy();
});

it("remove works for groups and orders", () => {
  const onChange = sheet({ ...base, groups: ["project"], order: [{ field: "due_date", reversed: false }] });
  fireEvent.click(screen.getByLabelText(`Remove ${name("project").name}`));
  expect(onChange).toHaveBeenLastCalledWith({ ...base, order: [{ field: "due_date", reversed: false }] });
  fireEvent.click(screen.getByLabelText(`Remove ${name("due_date").name}`));
  expect(onChange).toHaveBeenLastCalledWith({ ...base, groups: ["project"] });
});

it("Move up / Move down reorder and are disabled at the ends", () => {
  const onChange = sheet({ ...base, groups: ["project", "label"] });
  const p = name("project").name;
  const l = name("label").name;
  expect((screen.getByLabelText(`Move ${p} up`) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByLabelText(`Move ${l} down`) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByLabelText(`Move ${p} down`));
  expect(onChange).toHaveBeenCalledWith({ ...base, groups: ["label", "project"] });
  fireEvent.click(screen.getByLabelText(`Move ${l} up`));
  expect(onChange).toHaveBeenLastCalledWith({ ...base, groups: ["label", "project"] });
});

it("hides the add button when no choices remain", () => {
  const all = sortChoices().map((c) => c.field);
  sheet({ ...base, groups: all });
  expect(screen.queryByRole("button", { name: "Add group" })).toBeNull();
});

const panel = (onClose: () => void, run = vi.fn()) =>
  render(
    <ArrangePanel
      title="Sort"
      action={{ label: "Reset to default", run }}
      anchor={{ current: document.body }}
      onClose={onClose}
    >
      <p>body</p>
    </ArrangePanel>,
  );

it("ArrangePanel is a labelled dialog that closes on Escape", () => {
  const onClose = vi.fn();
  panel(onClose);
  expect(screen.getByRole("dialog", { name: "Sort" })).toBeTruthy();
  fireEvent.keyDown(document, { key: "Escape" });
  expect(onClose).toHaveBeenCalled();
});

it("ArrangePanel closes on a click on the scrim and runs its action", () => {
  const onClose = vi.fn();
  const run = vi.fn();
  const { container } = panel(onClose, run);
  fireEvent.click(screen.getByRole("button", { name: "Reset to default" }));
  expect(run).toHaveBeenCalled();
  expect(onClose).not.toHaveBeenCalled();
  const scrim = container.ownerDocument.querySelector(".arrange-scrim");
  if (!scrim) throw new Error("no scrim");
  fireEvent.click(scrim);
  expect(onClose).toHaveBeenCalled();
});
