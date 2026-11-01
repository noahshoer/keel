import { describe, expect, it } from "vitest";
import { partsOf, timeOf } from "./timeParts";

describe("the time wheel's columns", () => {
  it.each([
    ["17:05", { hour: 5, minute: 5, pm: true }],
    ["00:30", { hour: 12, minute: 30, pm: false }],
    ["12:00", { hour: 12, minute: 0, pm: true }],
    ["09:59", { hour: 9, minute: 59, pm: false }],
  ])("%s reads and writes back", (time, parts) => {
    expect(partsOf(time)).toEqual(parts);
    expect(timeOf(parts)).toBe(time);
  });
});
