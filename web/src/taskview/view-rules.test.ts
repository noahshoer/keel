import { expect, it } from "vitest";
import type { EditorValues, FieldLayout } from "../bindings";
import { fieldsShown, sameValues, settle } from "./view-rules";

const layout: FieldLayout = {
  fields: ["Date", "Location"],
  missing: [
    { missing: { kind: "Field", field: "Description" }, label: "+ Description" },
    { missing: { kind: "Repeat" }, label: "+ Repeat" },
  ],
};

it("an opened description becomes the first row and loses its pill", () => {
  expect(fieldsShown(layout, false)).toBe(layout);
  expect(fieldsShown(layout, true)).toEqual({
    fields: ["Description", "Date", "Location"],
    missing: [{ missing: { kind: "Repeat" }, label: "+ Repeat" }],
  });
  const set: FieldLayout = { fields: ["Description", "Date"], missing: [] };
  expect(fieldsShown(set, true)).toBe(set);
});

it("values compare by value", () => {
  const v: EditorValues = {
    content: "a",
    description: "",
    date: "",
    time: "",
    priority: "4",
    repeat: "none",
    project_id: "p",
    section_id: null,
    labels: [],
    parent_id: null,
    deadline: "",
    reminders: [],
    plain: [],
  };
  expect(sameValues(v, { ...v })).toBe(true);
  expect(sameValues(v, { ...v, priority: "1" })).toBe(false);
});

it("settle reports whether a call went through, never rejecting", async () => {
  expect(await settle(Promise.resolve(1))).toBe(true);
  expect(await settle(Promise.reject("disk full"))).toBe(false);
});
