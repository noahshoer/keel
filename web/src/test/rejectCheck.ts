// Makes the backend's `check_filter` reject with `message` (the save backstop), or pass when it's
// `null`, and records every dispatched action type.
export async function rejectFilterCheck(message: string | null): Promise<string[]> {
  const real = (await import("../ipc/mock")).mockHandlers as unknown as Record<
    string,
    (x: unknown) => Promise<unknown>
  >;
  const sent: string[] = [];
  window.__TAURI__ = {
    core: {
      invoke: (cmd, args) => {
        if (cmd === "check_filter") return message === null ? Promise.resolve(null) : Promise.reject(message);
        const a = (args as { args: { cmd?: { action: { type: string } } } }).args;
        if (cmd === "dispatch" && a.cmd) sent.push(a.cmd.action.type);
        return real[cmd]?.(a) ?? Promise.resolve(null);
      },
    },
  };
  return sent;
}
