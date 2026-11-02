// Text on a page that was just changed: the view it comes from arrives after a
// reload, so until then the page keeps showing what was entered.
import { useLayoutEffect, useState } from "react";

export function useHeldText<T extends string | null>(
  current: T,
): { text: T | string; hold: (next: string) => void } {
  const [held, setHeld] = useState<{ from: T; to: string } | null>(null);
  // Once the view moves on, this is over: the name may come back to `from` later.
  useLayoutEffect(() => {
    if (held && held.from !== current) setHeld(null);
  }, [held, current]);
  return {
    text: held && held.from === current ? held.to : current,
    hold: (next) => setHeld({ from: current, to: next }),
  };
}
