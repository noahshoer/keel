// The slice of Tauri's global API Keel uses (`app.withGlobalTauri` is on).
// Absent in a plain browser, where `invoke.ts` falls back to the dev mock.
export {};

declare global {
  interface Window {
    __TAURI__?: {
      core: {
        invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown>;
      };
    };
  }
}
