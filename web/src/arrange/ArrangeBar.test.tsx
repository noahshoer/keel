import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, expect, it, vi } from "vitest";
import type { Arrangement } from "../bindings";
import { arrangementSummary } from "../core";
import { loadCoreForTests } from "../test/wasm";
import { ArrangeBar } from "./ArrangeBar";

beforeAll(() => {
  loadCoreForTests();
});

const base: Arrangement = { filter: "", groups: [], order: [] };
const bar = (arrangement: Arrangement, problem: string | null = null) => {
  const fns = { onOpen: vi.fn(), onClearFilter: vi.fn(), onResetSort: vi.fn() };
  const r = render(<ArrangeBar arrangement={arrangement} problem={problem} {...fns} />);
  return { ...r, ...fns };
};

it("renders nothing when the page is not arranged", () => {
  const { container } = bar({ ...base, filter: "  " });
  expect(container.firstChild).toBeNull();
});

it("shows only the part that is set", () => {
  const { container } = bar({ ...base, filter: "p1" });
  expect(container.querySelectorAll(".arrange-chip")).toHaveLength(1);
  expect(screen.queryByLabelText("Reset sort")).toBeNull();
});

it("the sort chip says what keel-core summarizes", () => {
  const a: Arrangement = { ...base, order: [{ field: "priority", reversed: false }] };
  const { container } = bar(a);
  expect(arrangementSummary(a)).not.toBe("");
  expect(container.querySelector(".arrange-chip-text")?.textContent).toBe(arrangementSummary(a));
  expect(screen.queryByLabelText("Clear filter")).toBeNull();
});

it("the main part opens its panel and the cross clears, as sibling buttons", () => {
  const a: Arrangement = { filter: "p1", groups: [], order: [{ field: "priority", reversed: false }] };
  const { container, onOpen, onClearFilter, onResetSort } = bar(a);
  expect(container.querySelector("button button")).toBeNull();
  fireEvent.click(screen.getByText("p1"));
  expect(onOpen).toHaveBeenCalledWith("filter");
  fireEvent.click(screen.getByText(arrangementSummary(a)));
  expect(onOpen).toHaveBeenLastCalledWith("sort");
  fireEvent.click(screen.getByLabelText("Clear filter"));
  expect(onClearFilter).toHaveBeenCalledOnce();
  fireEvent.click(screen.getByLabelText("Reset sort"));
  expect(onResetSort).toHaveBeenCalledOnce();
});

it("a problem marks the chip and explains itself", () => {
  const { container } = bar({ ...base, filter: "p1 &" }, "expected a term after '&'");
  const chip = container.querySelector(".arrange-chip") as HTMLElement;
  expect(chip.classList.contains("problem")).toBe(true);
  expect(chip.title).toBe("expected a term after '&'");
});
