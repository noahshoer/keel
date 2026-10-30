// Never loads the wasm: proves the wrappers refuse to run before initCore().
import { expect, it } from "vitest";
import { dueLabel } from ".";

it("throws before init", () => {
  expect(() => dueLabel("2026-10-30", "2026-10-29")).toThrow(
    "keel core not initialised: await initCore() first",
  );
});
