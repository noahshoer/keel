import { describe, expect, it } from "vitest";
import type { View } from "../app/view";
import type { Label, PlacedArrangement, Project } from "../bindings";
import { arrangementOf, isArranged, NO_ARRANGEMENT, placeOf } from "./place";

const inbox = { id: "p-inbox", name: "Inbox", is_inbox: true } as Project;
const labels = [{ id: "l-errand", name: "errand" }] as Label[];
const loaded = { inbox, labels };

describe("placeOf", () => {
  it("maps each page to its place", () => {
    expect(placeOf({ kind: "inbox" }, loaded)).toEqual({ kind: "project", id: "p-inbox" });
    expect(placeOf({ kind: "today" }, loaded)).toEqual({ kind: "today" });
    expect(placeOf({ kind: "upcoming" }, loaded)).toEqual({ kind: "upcoming" });
    expect(placeOf({ kind: "project", id: "p1", name: "Home" }, loaded)).toEqual({
      kind: "project",
      id: "p1",
    });
    expect(placeOf({ kind: "filter", id: "f1", name: "F", query: "p1" }, loaded)).toEqual({
      kind: "filter",
      id: "f1",
    });
  });

  it("finds a label by name, ignoring case", () => {
    expect(placeOf({ kind: "label", name: "Errand" }, loaded)).toEqual({ kind: "label", id: "l-errand" });
  });

  it("has no place for Search or an unknown label", () => {
    expect(placeOf({ kind: "search" } satisfies View, loaded)).toBeNull();
    expect(placeOf({ kind: "label", name: "nope" }, loaded)).toBeNull();
  });
});

describe("arrangementOf", () => {
  const sorted = { filter: "", groups: ["priority"], order: [] } as PlacedArrangement["arrangement"];
  const all: PlacedArrangement[] = [{ place: { kind: "today" }, arrangement: sorted }];

  it("finds the arrangement by place", () => {
    expect(arrangementOf({ kind: "today" }, all)).toEqual(sorted);
  });

  it("falls back to none", () => {
    expect(arrangementOf({ kind: "upcoming" }, all)).toEqual(NO_ARRANGEMENT);
    expect(arrangementOf(null, all)).toEqual(NO_ARRANGEMENT);
  });

  it("isArranged is false only for the default", () => {
    expect(isArranged(NO_ARRANGEMENT)).toBe(false);
    expect(isArranged({ filter: "  ", groups: [], order: [] })).toBe(false);
    expect(isArranged(sorted)).toBe(true);
    expect(isArranged({ filter: "p1", groups: [], order: [] })).toBe(true);
  });
});
