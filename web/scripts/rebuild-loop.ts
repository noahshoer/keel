// The wasm watcher's rebuild loop (scripts/watch-wasm.ts), kept apart so it
// can be tested without cargo. Changes are debounced into one build; a change
// that lands during a build queues one more; a failed build is logged and the
// loop keeps going.

export type BuildResult = { ok: true } | { ok: false; output: string };

export type RebuildLoop = {
  /** Records a changed file and (re)starts the debounce. */
  trigger(file: string): void;
  /** Resolves once no build is pending or running. */
  whenIdle(): Promise<void>;
};

export function rebuildLoop(opts: {
  build: () => Promise<BuildResult>;
  log: (line: string) => void;
  delayMs: number;
  now?: () => number;
}): RebuildLoop {
  const now = opts.now ?? Date.now;
  const changed = new Set<string>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let running = false;
  let again = false;
  let idleWaiters: (() => void)[] = [];

  const busy = () => running || timer !== undefined;
  const settle = () => {
    if (busy()) return;
    const waiters = idleWaiters;
    idleWaiters = [];
    for (const w of waiters) w();
  };

  const summary = (files: string[]) =>
    files.length <= 3 ? files.join(", ") : `${files.slice(0, 3).join(", ")} +${files.length - 3} more`;

  async function run(): Promise<void> {
    timer = undefined;
    if (running) {
      again = true;
      return;
    }
    running = true;
    const files = [...changed];
    changed.clear();
    const start = now();
    let result: BuildResult;
    try {
      result = await opts.build();
    } catch (e) {
      result = { ok: false, output: String(e) };
    }
    const secs = ((now() - start) / 1000).toFixed(1);
    if (result.ok) {
      opts.log(`watch-wasm: rebuilt in ${secs}s (${summary(files)})`);
    } else {
      opts.log(
        `watch-wasm: build failed after ${secs}s (${summary(files)}); still watching\n${result.output.trimEnd()}`,
      );
    }
    running = false;
    if (again) {
      again = false;
      await run();
      return;
    }
    settle();
  }

  return {
    trigger(file) {
      changed.add(file);
      if (timer !== undefined) clearTimeout(timer);
      timer = setTimeout(() => void run(), opts.delayMs);
    },
    whenIdle() {
      if (!busy()) return Promise.resolve();
      return new Promise((resolve) => idleWaiters.push(resolve));
    },
  };
}
