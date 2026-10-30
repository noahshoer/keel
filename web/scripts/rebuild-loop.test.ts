import { expect, test } from "vitest";
import { type BuildResult, rebuildLoop } from "./rebuild-loop.ts";

function harness(results: BuildResult[]) {
  const lines: string[] = [];
  let builds = 0;
  let release: (() => void) | undefined;
  let hold = false;
  const loop = rebuildLoop({
    delayMs: 5,
    log: (l) => lines.push(l),
    now: () => 0,
    build: async () => {
      builds++;
      if (hold) await new Promise<void>((r) => (release = r));
      return results.shift() ?? { ok: true };
    },
  });
  return {
    loop,
    lines,
    builds: () => builds,
    hold: (on: boolean) => (hold = on),
    release: () => release?.(),
  };
}

const tick = (ms: number) => new Promise((r) => setTimeout(r, ms));

test("debounces a burst of changes into one build and logs one line", async () => {
  const h = harness([]);
  h.loop.trigger("crates/core/src/views.rs");
  h.loop.trigger("crates/core/src/views.rs");
  h.loop.trigger("crates/core/src/model.rs");
  await h.loop.whenIdle();
  expect(h.builds()).toBe(1);
  expect(h.lines).toEqual([
    "watch-wasm: rebuilt in 0.0s (crates/core/src/views.rs, crates/core/src/model.rs)",
  ]);
});

test("a failed build logs its error and the loop keeps watching", async () => {
  const h = harness([{ ok: false, output: "error[E0308]: mismatched types\n" }]);
  h.loop.trigger("crates/core/src/views.rs");
  await h.loop.whenIdle();
  expect(h.lines[0]).toBe(
    "watch-wasm: build failed after 0.0s (crates/core/src/views.rs); still watching\nerror[E0308]: mismatched types",
  );

  h.loop.trigger("crates/core/src/views.rs");
  await h.loop.whenIdle();
  expect(h.builds()).toBe(2);
  expect(h.lines[1]).toMatch(/^watch-wasm: rebuilt/);
});

test("a build that throws is logged like a failure", async () => {
  const h = harness([]);
  const loop = rebuildLoop({
    delayMs: 1,
    log: (l) => h.lines.push(l),
    now: () => 0,
    build: async () => {
      throw new Error("spawn cargo ENOENT");
    },
  });
  loop.trigger("a.rs");
  await loop.whenIdle();
  expect(h.lines[0]).toContain("Error: spawn cargo ENOENT");
});

test("a change during a build queues exactly one more build", async () => {
  const h = harness([]);
  h.hold(true);
  h.loop.trigger("a.rs");
  await tick(20);
  expect(h.builds()).toBe(1);
  h.loop.trigger("b.rs");
  h.loop.trigger("c.rs");
  await tick(20);
  h.hold(false);
  h.release();
  await h.loop.whenIdle();
  expect(h.builds()).toBe(2);
  expect(h.lines).toEqual(["watch-wasm: rebuilt in 0.0s (a.rs)", "watch-wasm: rebuilt in 0.0s (b.rs, c.rs)"]);
});
