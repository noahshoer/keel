import { expect, it } from "vitest";
import type { Label, SpanKind, TitleSegment } from "../bindings";
import { markClass, markStyle } from "./titleMarks";

function seg(kind: SpanKind, extra: Partial<TitleSegment> = {}): TitleSegment {
  return { text: "x", kind, level: 0, bucket: null, urgent: false, is_new: false, unknown: false, ...extra };
}

function label(name: string, color: string): Label {
  return { id: name, name, color, item_order: 0, is_favorite: false, is_deleted: false };
}

it("paints a date by its bucket, as the date pill is", () => {
  expect(markClass(seg("Date", { bucket: "Tomorrow" }))).toBe("hl hl-due-tomorrow");
  expect(markClass(seg("Time", { bucket: "ThisWeek" }))).toBe("hl hl-due-week");
  expect(markClass(seg("Date", { bucket: "Overdue" }))).toBe("hl hl-due-overdue");
  expect(markClass(seg("Date"))).toBe("hl hl-due-later");
});

it("paints a priority by its level", () => {
  expect(markClass(seg("Priority", { level: 1 }))).toBe("hl hl-p1");
  expect(markClass(seg("Priority", { level: 4 }))).toBe("hl hl-p4");
});

it("paints a deadline neutral until it is urgent, then red", () => {
  expect(markClass(seg("Deadline"))).toBe("hl hl-deadline");
  expect(markClass(seg("Deadline", { urgent: true }))).toBe("hl hl-deadline hl-urgent");
});

it("gives project, section, repeat and streak their own colors", () => {
  expect(markClass(seg("Project"))).toBe("hl hl-project");
  expect(markClass(seg("Section"))).toBe("hl hl-project");
  expect(markClass(seg("Repeat"))).toBe("hl hl-repeat");
  expect(markClass(seg("Streak"))).toBe("hl hl-streak");
});

it("paints a label in its own color, the accent when it has none or is new", () => {
  const labels = [label("home", "red"), label("plain", "charcoal")];
  expect(markStyle(seg("Label", { text: "@home" }), labels)).toEqual({ "--hl-color": "var(--label-red)" });
  expect(markStyle(seg("Label", { text: "@HOME" }), labels)).toEqual({ "--hl-color": "var(--label-red)" });
  expect(markStyle(seg("Label", { text: "@plain" }), labels)).toBeUndefined();
  expect(markStyle(seg("Label", { text: "@new" }), labels)).toBeUndefined();
  expect(markStyle(seg("Date"), labels)).toBeUndefined();
});
