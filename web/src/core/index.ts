// keel-core's data rules in the browser (crates/core-wasm). The only module
// that imports ./pkg (Biome noRestrictedImports): everything else calls these
// wrappers, which refuse to run until initCore() has loaded the wasm.
import type { Card, CardMode, Id, Progress, Task } from "../bindings";
import init, * as core from "./pkg/keel_core_wasm.js";

let initialised = false;

/** Loads the wasm; await it once before mounting the app. */
export async function initCore(): Promise<void> {
  if (initialised) return;
  await init();
  initialised = true;
}

/** Loads the wasm from its bytes, synchronously (tests: src/test/wasm.ts). */
export function initCoreSync(bytes: BufferSource): void {
  if (initialised) return;
  core.initSync({ module: bytes });
  initialised = true;
}

function ready(): void {
  if (!initialised) throw new Error("keel core not initialised: await initCore() first");
}

/** Due chip text ("Today", "Friday", "3 Nov"). Dates are `YYYY-MM-DD` (or a `Due.date` datetime). */
export function dueLabel(due: string, today: string): string {
  ready();
  return core.dueLabel(due, today);
}

/** 12-hour label ("5 PM", "5:30 PM") for an `HH:MM[:SS]` time. */
export function timeLabel(time: string): string {
  ready();
  return core.timeLabel(time);
}

/** A parent's direct sub-task counts; `null` when nothing is counted. */
export function progress(parent: Id, all: Task[]): Progress | null {
  ready();
  return core.progress(parent, all);
}

/** Groups a view's matched tasks into cards (sub-tasks under their parent). */
export function cards(matched: Task[], all: Task[], mode: CardMode): Card[] {
  ready();
  return core.cards(matched, all, mode);
}
