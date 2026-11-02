import { describe, expect, it } from "vitest";
import { filledPill, pillsRunBack, pillsRunOn, revealLeft } from "./pillFade";

it("runs back once the row is scrolled from its start", () => {
  expect(pillsRunBack(0)).toBe(false);
  expect(pillsRunBack(40)).toBe(true);
});

it("is true while a pill runs past the end of the row", () => {
  expect(pillsRunOn(0, 300, 700)).toBe(true);
  expect(pillsRunOn(100, 300, 700)).toBe(true);
});

it("is false once the last pill has cleared it, or when they all fit", () => {
  expect(pillsRunOn(400, 300, 700)).toBe(false);
  expect(pillsRunOn(0, 300, 300)).toBe(false);
});

describe("filledPill", () => {
  it("is the pill a phrase just filled", () => {
    expect(filledPill({}, { Priority: "P1" })).toBe("Priority");
    expect(filledPill({ Date: "Tomorrow" }, { Date: "Tomorrow", Labels: "phone" })).toBe("Labels");
  });

  it("is a pill whose phrase changed what it says", () => {
    expect(filledPill({ Date: "Tomorrow" }, { Date: "Tomorrow 5 PM" })).toBe("Date");
  });

  it("is nothing when a phrase goes or nothing changed", () => {
    expect(filledPill({ Priority: "P1" }, {})).toBeNull();
    expect(filledPill({ Priority: "P1" }, { Priority: "P1" })).toBeNull();
  });

  it("is the last one, when several fill at once", () => {
    expect(filledPill({}, { Date: "Today", Priority: "P1" })).toBe("Priority");
  });
});

describe("revealLeft", () => {
  it("stays put for a pill that's already in view", () => {
    expect(revealLeft(50, 16, 340, 100, 200)).toBe(50);
  });

  it("scrolls on just far enough for a pill past the end", () => {
    expect(revealLeft(50, 16, 340, 300, 420)).toBe(130);
  });

  it("scrolls back just far enough for a pill past the start", () => {
    expect(revealLeft(50, 16, 340, -20, 60)).toBe(14);
  });

  it("shows the start of a pill too wide for the room", () => {
    expect(revealLeft(0, 16, 340, 100, 600)).toBe(84);
  });
});
