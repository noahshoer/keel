/// <reference types="node" />
// Loads keel-core's wasm for a Vitest file: call loadCoreForTests() in
// beforeAll. Deliberately not a global setup, so a test file can check the
// uninitialised behaviour (src/core/core-init.test.ts). Needs `pnpm build:wasm`.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { initCoreSync } from "../core";

export function loadCoreForTests(): void {
  // import.meta.url is not a file: URL under jsdom; dirname is the file's directory.
  initCoreSync(readFileSync(join(import.meta.dirname, "../core/pkg/keel_core_wasm_bg.wasm")));
}
