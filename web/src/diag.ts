// Diagnostics logging from the page, ported 1:1 from crates/ui/src/diag.rs.
// Off by default: the app turns it on from the Settings switch or
// `KEEL_DIAG=1` (see crates/src-tauri/src/diag.rs), and init() asks at
// startup. While off, nothing is sent and no hooks are installed. While on,
// log("what happened") goes to the same diag.log as Rust's and Swift's lines,
// and JS errors, unhandled rejections and console.error are captured.
//
// Sources match diag.rs: "js", "js-error", "js-rejection", "console.error",
// "viewport". (diag.rs's "wasm-panic" came from the Dioxus wasm's panic hook;
// a keel-core wasm panic here surfaces as a thrown RuntimeError, which the
// error hooks capture.)

let active = false;
let installed = false;

// Straight to Tauri, not through ipc/invoke: in a plain browser this is a
// no-op (as in diag.rs), rather than the dev mock.
function send(source: string, msg: unknown): void {
  if (!active) return;
  try {
    window.__TAURI__?.core.invoke("diag_log", { args: { source, msg: String(msg) } }).catch(() => {});
  } catch {
    // Logging must never break the page.
  }
}

/**
 * A value as one log line. Never throws: an object seen again prints
 * `[Circular]`, a BigInt prints `10n`, and anything JSON can't handle falls
 * back to `String(x)`. Same rules as the hook in crates/ui/src/diag.rs.
 */
export function describe(x: unknown): string {
  if (x instanceof Error && x.stack) return x.stack;
  if (typeof x === "bigint") return `${x}n`;
  if (typeof x === "object" && x !== null) {
    const stack = (x as { stack?: unknown }).stack;
    if (typeof stack === "string" && stack) return stack;
    const seen = new WeakSet<object>();
    try {
      return JSON.stringify(x, (_key, v: unknown) => {
        if (typeof v === "bigint") return `${v}n`;
        if (typeof v === "object" && v !== null) {
          if (seen.has(v)) return "[Circular]";
          seen.add(v);
        }
        return v;
      });
    } catch {
      return String(x);
    }
  }
  return String(x);
}

function install(): void {
  window.addEventListener("error", (e) => {
    const stack = e.error instanceof Error && e.error.stack ? `\n${e.error.stack}` : "";
    send("js-error", `${e.message} at ${e.filename}:${e.lineno}:${e.colno}${stack}`);
  });
  window.addEventListener("unhandledrejection", (e) => {
    const r: unknown = e.reason;
    const { stack, message } = (typeof r === "object" && r !== null ? r : {}) as {
      stack?: unknown;
      message?: unknown;
    };
    send(
      "js-rejection",
      (typeof stack === "string" && stack) || (typeof message === "string" && message) || String(r),
    );
  });
  const err = console.error;
  console.error = (...a: unknown[]) => {
    send("console.error", a.map(describe).join(" "));
    err.apply(console, a);
  };
  window.visualViewport?.addEventListener("resize", () => {
    const v = window.visualViewport;
    if (!v) return;
    send(
      "viewport",
      `resize ${v.width.toFixed(0)}x${v.height.toFixed(1)} top ${v.offsetTop.toFixed(1)} scale ${v.scale}`,
    );
  });
}

/** Adds a line to the diagnostics log (a no-op while logging is off). */
export function log(msg: string): void {
  send("js", msg);
}

/** Turns the page's side of logging on or off; the first time it's on, the hooks are installed. */
export function setActive(on: boolean): void {
  active = on;
  if (on) send("js", `diagnostics on; ${navigator.userAgent}; inner ${innerWidth}x${innerHeight}`);
  if (!on || installed) return;
  installed = true;
  install();
}

/** Asks the app whether logging is on (call once at startup). */
export async function init(): Promise<void> {
  try {
    const on = await window.__TAURI__?.core.invoke("diag_enabled", { args: {} });
    setActive(on === true);
  } catch {
    // As diag.rs: an error leaves logging off.
  }
}
