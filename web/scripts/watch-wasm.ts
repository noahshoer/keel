// Rebuilds keel-core's wasm (scripts/build-wasm.ts) whenever a file under
// crates/core/src or crates/core-wasm/src changes, one log line per rebuild.
// A failed build logs its error and the watcher keeps going. Started by
// scripts/dev.ts (`pnpm dev`); it exits when dev.ts goes away (the IPC
// channel closes).
import { spawn } from "node:child_process";
import { watch } from "node:fs";
import { relative } from "node:path";
import { fileURLToPath } from "node:url";
import { type BuildResult, rebuildLoop } from "./rebuild-loop.ts";

const repo = fileURLToPath(new URL("../../", import.meta.url));
const buildScript = fileURLToPath(new URL("./build-wasm.ts", import.meta.url));
const dirs = ["crates/core/src", "crates/core-wasm/src"];

function build(): Promise<BuildResult> {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [buildScript], { stdio: ["ignore", "pipe", "pipe"] });
    let output = "";
    child.stdout.on("data", (d: Buffer) => (output += d.toString()));
    child.stderr.on("data", (d: Buffer) => (output += d.toString()));
    child.on("error", (e) => resolve({ ok: false, output: String(e) }));
    child.on("close", (code) => resolve(code === 0 ? { ok: true } : { ok: false, output }));
  });
}

const loop = rebuildLoop({
  build,
  delayMs: 300,
  log: (line) => console.log(line),
});

for (const dir of dirs) {
  const abs = `${repo}${dir}`;
  watch(abs, { recursive: true }, (_event, name) => {
    if (name === null) return;
    loop.trigger(relative(repo, `${abs}/${name}`).split("\\").join("/"));
  });
}
console.log(`watch-wasm: watching ${dirs.join(", ")}`);

// Started with an IPC channel by dev.ts: when that process ends (however it
// ends), the channel closes and the watcher stops with it.
process.on("disconnect", () => process.exit(0));
