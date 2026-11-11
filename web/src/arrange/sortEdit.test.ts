import { expect, it } from "vitest";
import type { Arrangement, SortChoice } from "../bindings";
import {
  addGroup,
  addOrder,
  flipOrder,
  moveGroup,
  moveOrder,
  removeGroup,
  removeOrder,
  resetSort,
  unusedGroups,
  unusedOrders,
} from "./sortEdit";

const base: Arrangement = {
  filter: "p1",
  groups: ["label", "project"],
  order: [
    { field: "due_date", reversed: false },
    { field: "priority", reversed: true },
  ],
};
const choice = (field: SortChoice["field"]): SortChoice => ({
  field,
  name: field,
  order_label: "up",
  reversed_label: "down",
});
const choices = (["label", "project", "due_date", "priority"] as const).map(choice);

it("addGroup appends", () => {
  expect(addGroup(base, "due_date").groups).toEqual(["label", "project", "due_date"]);
});
it("addGroup ignores a field already used", () => {
  expect(addGroup(base, "label")).toEqual(base);
});
it("removeGroup drops the index and ignores out of range", () => {
  expect(removeGroup(base, 0).groups).toEqual(["project"]);
  expect(removeGroup(base, 5)).toEqual(base);
});
it("moveGroup moves, and to the same index changes nothing", () => {
  expect(moveGroup(base, 0, 1).groups).toEqual(["project", "label"]);
  expect(moveGroup(base, 1, 1)).toEqual(base);
  expect(moveGroup(base, 0, 9)).toEqual(base);
  expect(moveGroup(base, -1, 0)).toEqual(base);
});
it("addOrder appends ascending", () => {
  const a = addOrder(base, "label");
  expect(a.order[2]).toEqual({ field: "label", reversed: false });
});
it("addOrder ignores a field already used", () => {
  expect(addOrder(base, "due_date")).toEqual(base);
});
it("removeOrder and moveOrder", () => {
  expect(removeOrder(base, 0).order.map((o) => o.field)).toEqual(["priority"]);
  expect(moveOrder(base, 1, 0).order.map((o) => o.field)).toEqual(["priority", "due_date"]);
  expect(moveOrder(base, 0, 0)).toEqual(base);
  expect(moveOrder(base, 0, 2)).toEqual(base);
});
it("flipOrder flips one level", () => {
  expect(flipOrder(base, 0).order[0]).toEqual({ field: "due_date", reversed: true });
  expect(flipOrder(base, 1).order[1]).toEqual({ field: "priority", reversed: false });
  expect(flipOrder(base, 7)).toEqual(base);
});
it("unused lists exclude used fields", () => {
  expect(unusedGroups(base, choices).map((c) => c.field)).toEqual(["due_date", "priority"]);
  expect(unusedOrders(base, choices).map((c) => c.field)).toEqual(["label", "project"]);
});
it("edits do not mutate the input", () => {
  const copy = structuredClone(base);
  addGroup(base, "due_date");
  moveOrder(base, 0, 1);
  flipOrder(base, 0);
  expect(base).toEqual(copy);
});
it("resetSort keeps the filter", () => {
  expect(resetSort(base)).toEqual({ filter: "p1", groups: [], order: [] });
});
