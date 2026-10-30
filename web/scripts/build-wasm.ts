// Builds crates/core-wasm for the browser into src/core/pkg (git-ignored):
// cargo → wasm32 cdylib, then the project-local wasm-bindgen CLI
// (.tools/bin/wasm-bindgen, never one on PATH) whose version must equal the
// crate's pinned `wasm-bindgen = "=X"`. Run with `pnpm build:wasm`.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("../../", import.meta.url));
const crateToml = `${repo}crates/core-wasm/Cargo.toml`;
const cli = `${repo}.tools/bin/wasm-bindgen${process.platform === "win32" ? ".exe" : ""}`;
const wasm = `${repo}target/wasm32-unknown-unknown/release/keel_core_wasm.wasm`;
const outDir = fileURLToPath(new URL("../src/core/pkg", import.meta.url));

function fail(message: string): never {
  console.error(`build-wasm: ${message}`);
  process.exit(1);
}

const pinned = /^wasm-bindgen = "=([^"]+)"/m.exec(readFileSync(crateToml, "utf8"))?.[1];
if (pinned === undefined) fail(`${crateToml} must pin wasm-bindgen exactly (wasm-bindgen = "=X.Y.Z")`);

const install = `cargo install wasm-bindgen-cli --version ${pinned} --locked --root .tools (from the repo root)`;
if (!existsSync(cli)) fail(`${cli} is missing; install it with: ${install}`);
const version = execFileSync(cli, ["--version"], { encoding: "utf8" }).trim();
if (version !== `wasm-bindgen ${pinned}`) {
  fail(`${cli} is "${version}", but crates/core-wasm pins ${pinned}; reinstall with: ${install}`);
}

// The command's own output (cargo's errors) is already on the terminal, so a
// failure only adds one line instead of Node's stack trace.
const run = (cmd: string, args: string[]) => {
  try {
    execFileSync(cmd, args, { cwd: repo, stdio: "inherit" });
  } catch {
    fail(`${cmd} ${args[0] ?? ""} failed`);
  }
};
run("cargo", ["build", "-p", "keel-core-wasm", "--target", "wasm32-unknown-unknown", "--release"]);
run(cli, ["--target", "web", "--out-dir", outDir, "--out-name", "keel_core_wasm", wasm]);
