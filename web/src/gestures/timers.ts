// Timeouts that remove themselves when they fire and can all be cancelled at once.

export type Timers = {
  /** Runs `f` after `ms`, unless `clear` comes first. */
  later: (f: () => void, ms: number) => void;
  /** Cancels every timeout still waiting. */
  clear: () => void;
};

export function makeTimers(): Timers {
  const pending = new Set<ReturnType<typeof setTimeout>>();
  return {
    later: (f, ms) => {
      const t = setTimeout(() => {
        pending.delete(t);
        f();
      }, ms);
      pending.add(t);
    },
    clear: () => {
      for (const t of pending) clearTimeout(t);
      pending.clear();
    },
  };
}
