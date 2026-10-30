// `pnpm dev` (after the first build:wasm): the Vite dev server, in this
// process, plus scripts/watch-wasm.ts as a child that rebuilds keel-core's
// wasm on Rust changes. Vite reloads the page when the new src/core/pkg
// lands (the wasm and its glue are in its module graph). Stopping this
// process (Ctrl+C, a signal from `cargo tauri dev`, or Vite's `q`) stops the
// watcher too: it holds an IPC channel to us and exits when that closes.
import { fork } from "node:child_process";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const web = fileURLToPath(new URL("../", import.meta.url));

const server = await createServer({ root: web });
await server.listen();
server.printUrls();
server.bindCLIShortcuts({ print: true });

const watcher = fork(fileURLToPath(new URL("./watch-wasm.ts", import.meta.url)), [], { stdio: "inherit" });
watcher.on("exit", (code, signal) => {
  if (!stopping) console.error(`dev: the wasm watcher stopped (${signal ?? code}); Vite keeps running`);
});

let stopping = false;
async function stop(): Promise<void> {
  if (stopping) return;
  stopping = true;
  watcher.kill();
  await server.close();
  process.exit(0);
}
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
  process.on(signal, () => void stop());
}
