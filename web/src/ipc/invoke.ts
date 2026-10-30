import type { CommandName, Commands } from "./commands";

/**
 * Call a Tauri command. The payload is `{ args }` (every command takes one
 * `args` parameter, see `crates/src-tauri/src/args.rs`).
 *
 * Errors: Tauri rejects with the command's `Err(String)` value, and this
 * passes that string through untouched, so callers `catch (e)` and get the
 * backend's message as a plain string (not an Error).
 *
 * Without `window.__TAURI__` (a plain browser), dev builds use the in-memory
 * mock; a production build throws. The mock import sits behind
 * `import.meta.env.DEV`, so Vite drops it from production bundles.
 */
export async function invoke<K extends CommandName>(
  cmd: K,
  args: Commands[K]["args"],
): Promise<Commands[K]["ret"]> {
  const tauri = window.__TAURI__;
  if (tauri) {
    // Safe: `Commands` is the single source of truth for what each command returns.
    return (await tauri.core.invoke(cmd, { args })) as Commands[K]["ret"];
  }
  if (import.meta.env.DEV) {
    const { mockHandlers } = await import("./mock");
    return mockHandlers[cmd](args);
  }
  throw new Error("Keel must run inside its app");
}
