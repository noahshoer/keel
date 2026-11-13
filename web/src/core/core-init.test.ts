// Never loads the wasm: proves a core function fails (rather than answering
// wrongly) when called before initCore().
import { expect, it } from "vitest";
import { dueLabel } from ".";

it("throws before init", () => {
  expect(() => dueLabel("2026-10-30", "2026-10-29")).toThrow();
});
