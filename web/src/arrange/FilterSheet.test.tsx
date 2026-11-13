import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, expect, it, vi } from "vitest";
import type { Filter } from "../bindings";
import { loadCoreForTests } from "../test/wasm";
import { FilterSheet } from "./FilterSheet";

const { addFilter } = vi.hoisted(() => ({ addFilter: vi.fn(() => Promise.resolve()) }));
vi.mock("../ipc/actions", () => ({ addFilter }));

beforeAll(() => {
  loadCoreForTests();
});
beforeEach(() => {
  addFilter.mockClear();
});

const saved = (id: string, name: string, query: string): Filter => ({
  id,
  name,
  query,
  color: "",
  item_order: 0,
  is_favorite: false,
  is_deleted: false,
});

const setup = (over: Partial<Parameters<typeof FilterSheet>[0]> = {}) => {
  const props = {
    pageName: "Home",
    initial: "",
    names: { projects: [], sections: [], labels: [] },
    filters: [] as Filter[],
    counts: { shown: 3, total: 5 },
    anchor: { current: document.body },
    onDraft: vi.fn(),
    onSave: vi.fn(),
    onClose: vi.fn(),
    ...over,
  };
  render(<FilterSheet {...props} />);
  return props;
};
const field = () => screen.getByLabelText("Query") as HTMLTextAreaElement;
const type = (text: string) => fireEvent.change(field(), { target: { value: text } });
const enter = () => fireEvent.keyDown(field(), { key: "Enter" });

it("is titled with the page, and counts matches", () => {
  setup();
  expect(screen.getByRole("dialog", { name: "Filter Home" })).toBeTruthy();
  expect(screen.getByText("3 of 5 tasks match")).toBeTruthy();
});

it("a valid draft is previewed", () => {
  const { onDraft } = setup();
  type("p1");
  expect(onDraft).toHaveBeenLastCalledWith("p1");
});

it("an invalid draft shows its problem and previews nothing", () => {
  const { onDraft } = setup();
  type("p1 &");
  expect(document.querySelector(".filter-problem")).not.toBeNull();
  expect(onDraft).toHaveBeenLastCalledWith(null);
});

it("a blank draft previews as no filter", () => {
  const { onDraft } = setup({ initial: "p1" });
  type("");
  expect(onDraft).toHaveBeenLastCalledWith("");
});

it("Enter with a valid changed draft saves once, then closes", async () => {
  const { onSave, onClose } = setup();
  type("p1");
  enter();
  await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
  expect(onSave).toHaveBeenCalledTimes(1);
  expect(onSave).toHaveBeenCalledWith("p1");
});

it("Enter with an unchanged draft just closes", async () => {
  const { onSave, onClose } = setup({ initial: "p1" });
  enter();
  await waitFor(() => expect(onClose).toHaveBeenCalled());
  expect(onSave).not.toHaveBeenCalled();
});

it("closing with an invalid draft does not save", () => {
  const { onSave, onClose } = setup();
  type("p1 &");
  fireEvent.keyDown(document, { key: "Escape" });
  expect(onClose).toHaveBeenCalled();
  expect(onSave).not.toHaveBeenCalled();
});

it("closing with a valid changed draft saves it", () => {
  const { onSave, onClose } = setup();
  type("p2");
  fireEvent.keyDown(document, { key: "Escape" });
  expect(onSave).toHaveBeenCalledWith("p2");
  expect(onClose).toHaveBeenCalledTimes(1);
});

it("tapping a saved filter puts its query in the field and previews it", () => {
  const { onDraft } = setup({ filters: [saved("f1", "Urgent", "p1 & today")] });
  fireEvent.click(screen.getByRole("button", { name: /Urgent/ }));
  expect(field().value).toBe("p1 & today");
  expect(onDraft).toHaveBeenLastCalledWith("p1 & today");
  expect(screen.getByText("Your filters")).toBeTruthy();
});

it("each saved filter shows its query after its name", () => {
  setup({ filters: [saved("f1", "Urgent", "p1 & today")] });
  const row = screen.getByRole("button", { name: /Urgent/ });
  expect(row.textContent).toBe("Urgentp1 & today");
  expect(row.querySelector(".filter-row-query")?.textContent).toBe("p1 & today");
});

it("Save as a filter is hidden for a blank, invalid or already saved draft", () => {
  setup({ filters: [saved("f1", "Urgent", "p1")] });
  expect(screen.queryByRole("button", { name: "Save as a filter" })).toBeNull();
  type("p1 &");
  expect(screen.queryByRole("button", { name: "Save as a filter" })).toBeNull();
  type("p1");
  expect(screen.queryByRole("button", { name: "Save as a filter" })).toBeNull();
  type("p2");
  expect(screen.getByRole("button", { name: "Save as a filter" })).toBeTruthy();
});

it("Save as a filter asks for a name and creates the filter", async () => {
  const onFilterAdded = vi.fn();
  setup({ onFilterAdded });
  type("p2");
  fireEvent.click(screen.getByRole("button", { name: "Save as a filter" }));
  const name = screen.getByPlaceholderText("Filter name") as HTMLInputElement;
  fireEvent.change(name, { target: { value: "Second" } });
  await act(async () => {
    fireEvent.keyDown(name, { key: "Enter" });
  });
  expect(addFilter).toHaveBeenCalledWith("Second", "p2");
  await waitFor(() => expect(onFilterAdded).toHaveBeenCalled());
});

it("Clear saves an empty filter and closes", () => {
  const { onSave, onClose } = setup({ initial: "p1" });
  fireEvent.click(screen.getByRole("button", { name: "Clear" }));
  expect(onSave).toHaveBeenCalledWith("");
  expect(onClose).toHaveBeenCalledTimes(1);
});

it("a filter that can't be created says why and keeps the name", async () => {
  addFilter.mockImplementationOnce(() => Promise.reject("Name taken."));
  const onFilterAdded = vi.fn();
  setup({ onFilterAdded });
  type("p2");
  fireEvent.click(screen.getByRole("button", { name: "Save as a filter" }));
  const name = screen.getByPlaceholderText("Filter name") as HTMLInputElement;
  fireEvent.change(name, { target: { value: "Second" } });
  await act(async () => {
    fireEvent.keyDown(name, { key: "Enter" });
  });
  await waitFor(() => expect(screen.getByText("Couldn’t save the filter: Name taken.")).toBeTruthy());
  expect((screen.getByPlaceholderText("Filter name") as HTMLInputElement).value).toBe("Second");
  expect(onFilterAdded).not.toHaveBeenCalled();
});
