import { expect, it } from "vitest";
import { allTicked, countText, groupState, toggleGroup, unticked } from "./newNames";

it("a group is all, some or none ticked", () => {
  expect(groupState({ a: true, b: true })).toBe("all");
  expect(groupState({ a: true, b: false })).toBe("some");
  expect(groupState({ a: false, b: false })).toBe("none");
});

it("the header ticks all unless all are ticked, then unticks all", () => {
  expect(toggleGroup({ a: true, b: false })).toEqual({ a: true, b: true });
  expect(toggleGroup({ a: false, b: false })).toEqual({ a: true, b: true });
  expect(toggleGroup({ a: true, b: true })).toEqual({ a: false, b: false });
});

it("every name starts ticked", () => {
  expect(allTicked({ projects: ["wrok"], labels: ["errands", "office"] })).toEqual({
    projects: { wrok: true },
    labels: { errands: true, office: true },
  });
});

it("the unticked names, by kind", () => {
  expect(unticked({ projects: {}, labels: { errands: true, office: false } })).toEqual({
    projects: [],
    labels: ["office"],
  });
});

it("counts the ticked names", () => {
  expect(countText({ errands: true, office: false })).toBe("1 of 2");
});
