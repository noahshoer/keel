import { afterEach, beforeEach, expect, test, vi } from "vitest";

// diag.ts keeps module state (on, hooks installed), so each test imports a
// fresh copy.
type Diag = typeof import("./diag");
let calls: { cmd: string; args: Record<string, unknown> | undefined }[];
let diagOn: boolean;
const originalError = console.error;

beforeEach(() => {
  vi.resetModules();
  calls = [];
  diagOn = false;
  window.__TAURI__ = {
    core: {
      invoke: async (cmd, args) => {
        calls.push({ cmd, args });
        return cmd === "diag_enabled" ? diagOn : null;
      },
    },
  };
});

afterEach(() => {
  console.error = originalError;
  delete window.__TAURI__;
});

const logged = () => calls.filter((c) => c.cmd === "diag_log").map((c) => c.args?.["args"]);
const load = async (): Promise<Diag> => import("./diag");

test("while off, nothing is sent and no hooks are installed", async () => {
  const diag = await load();
  const listen = vi.spyOn(window, "addEventListener");
  await diag.init();
  diag.log("hello");
  expect(console.error).toBe(originalError);
  expect(listen).not.toHaveBeenCalled();
  expect(logged()).toEqual([]);
  listen.mockRestore();
});

test("when on, it logs and captures errors, rejections and console.error", async () => {
  diagOn = true;
  const diag = await load();
  const silence = vi.fn();
  console.error = silence;
  await diag.init();
  expect(logged()[0]).toMatchObject({
    source: "js",
    msg: expect.stringMatching(/^diagnostics on; .*; inner \d+x\d+$/),
  });

  diag.log("hello");
  console.error("boom", { a: 1 });
  window.dispatchEvent(new ErrorEvent("error", { message: "bad", filename: "app.js", lineno: 3, colno: 7 }));
  const rejection = new Event("unhandledrejection") as Event & { reason?: unknown };
  rejection.reason = new Error("nope");
  window.dispatchEvent(rejection);

  const rest = logged().slice(1);
  expect(rest[0]).toEqual({ source: "js", msg: "hello" });
  expect(rest[1]).toEqual({ source: "console.error", msg: 'boom {"a":1}' });
  expect(silence).toHaveBeenCalledWith("boom", { a: 1 });
  expect(rest[2]).toEqual({ source: "js-error", msg: "bad at app.js:3:7" });
  expect(rest[3]).toMatchObject({ source: "js-rejection", msg: expect.stringContaining("nope") });
});

test("switching off stops sending; switching on again does not install twice", async () => {
  const diag = await load();
  diag.setActive(true);
  const wrapped = console.error;
  diag.setActive(false);
  diag.log("quiet");
  expect(logged().some((a) => a !== undefined && (a as { msg: string }).msg === "quiet")).toBe(false);
  diag.setActive(true);
  expect(console.error).toBe(wrapped);
});
