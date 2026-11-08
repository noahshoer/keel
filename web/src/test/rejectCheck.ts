// Makes the backend's `check_filter` reject (the save backstop) and records every dispatched action type.
export async function rejectFilterCheck(message: string): Promise<string[]> {
  const real = (await import("../ipc/mock")).mockHandlers as unknown as Record<
    string,
    (x: unknown) => Promise<unknown>
  >;
  const sent: string[] = [];
  window.__TAURI__ = {
    core: {
      invoke: (cmd, args) => {
        if (cmd === "check_filter") return Promise.reject(message);
        const a = (args as { args: { cmd?: { action: { type: string } } } }).args;
        if (cmd === "dispatch" && a.cmd) sent.push(a.cmd.action.type);
        return real[cmd]?.(a) ?? Promise.resolve(null);
      },
    },
  };
  return sent;
}
