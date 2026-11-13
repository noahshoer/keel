// initCore() is safe to call from several places at once: the wasm loads once.
import { expect, it, vi } from "vitest";

const init = vi.fn(async () => {});
vi.mock("./pkg/keel_core_wasm.js", () => ({ default: init, initSync: vi.fn() }));

it("concurrent initCore calls load the wasm once", async () => {
  // The test setup may already have imported this module (through the mock backend) before the
  // mock above applied; a fresh registry gets the copy that uses it.
  vi.resetModules();
  const { initCore } = await import(".");
  await Promise.all([initCore(), initCore()]);
  expect(init).toHaveBeenCalledTimes(1);
});
