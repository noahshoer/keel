import { beforeAll, describe, expect, it } from "vitest";
import * as fx from "../ipc/fixtures";
import { loadCoreForTests } from "../test/wasm";
import { bottomLayout, hiddenCount, locationsOf, ringDash, subtaskState } from "./rowLayout";

beforeAll(() => {
  loadCoreForTests();
});

describe("bottomLayout", () => {
  it.each([
    [false, false, "one-row"],
    [true, false, "one-row"],
    [false, true, "one-row"],
    [true, true, "two-rows"],
  ] as const)("when %s, labels %s: %s", (hasWhen, hasLabels, want) => {
    expect(bottomLayout(hasWhen, hasLabels)).toBe(want);
  });
});

describe("subtaskState", () => {
  it.each([
    [0, 4, "none"],
    [2, 5, "some"],
    [3, 3, "all"],
  ] as const)("%i of %i is %s", (done, total, want) => {
    expect(subtaskState({ done, total })).toBe(want);
  });
});

describe("ringDash", () => {
  it("fills the ring's edge by the share done", () => {
    expect(ringDash({ done: 2, total: 5 })).toBe("241 603");
  });
});

describe("hiddenCount", () => {
  it("counts the labels that start past the visible edge", () => {
    expect(hiddenCount([0, 40, 110, 140], 100)).toBe(2);
    expect(hiddenCount([0, 40], 100)).toBe(0);
    expect(hiddenCount([0, 110, 220], 100)).toBe(2);
  });

  it("counts a label with too little showing to read", () => {
    expect(hiddenCount([0, 40, 90], 100)).toBe(1);
    expect(hiddenCount([0, 40, 70], 100)).toBe(0);
  });

  it("always leaves the first label showing", () => {
    expect(hiddenCount([0], 0)).toBe(0);
    expect(hiddenCount([0, 10], 0)).toBe(1);
  });
});

describe("locationsOf", () => {
  const inbox = fx.projects.find((p) => p.is_inbox);
  const other = fx.projects.find((p) => !p.is_inbox);
  const base = fx.tasks[0];
  if (!inbox || !other || !base) throw new Error("fixtures");

  it("names the project, and the section when there is one", () => {
    const section = fx.sections.find((s) => s.project_id === other.id);
    const inSection = { ...base, id: "a", project_id: other.id, section_id: section?.id ?? null };
    const plain = { ...base, id: "b", project_id: other.id, section_id: null };
    const map = locationsOf([inSection, plain], fx.projects, fx.sections);
    expect(map.get("a")).toEqual({ project: other.name, section: section?.name ?? null });
    expect(map.get("b")).toEqual({ project: other.name, section: null });
  });

  it("falls back to the Inbox's name for a project it can't find", () => {
    const lost = { ...base, id: "c", project_id: "gone", section_id: null };
    expect(locationsOf([lost], fx.projects, fx.sections).get("c")).toEqual({
      project: inbox.name,
      section: null,
    });
  });
});
