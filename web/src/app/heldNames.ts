// A project, label or filter's new name, until the reload brings it. The change
// reloads everything, tens of milliseconds later, and the old name shouldn't come
// back in between. A save that fails lets go, so the old name returns.
import { useCallback, useEffect, useMemo, useState } from "react";
import type { Id } from "../bindings";
import type { Loaded } from "./load";

type Held = ReadonlyMap<Id, string>;

function named<T extends { id: Id; name: string }>(list: T[], held: Held): T[] {
  return list.map((item) => {
    const name = held.get(item.id);
    return name === undefined ? item : { ...item, name };
  });
}

/**
 * `sidebar` shows every held name. `page` holds back labels': a label's page
 * finds it by name, and its view has the old one until the same reload.
 */
export function useHeldNames(loaded: Loaded | null): {
  page: Loaded | null;
  sidebar: Loaded | null;
  holdName: (id: Id, name: string, change: Promise<unknown>) => void;
} {
  const [held, setHeld] = useState<Held>(new Map());

  const holdName = useCallback((id: Id, name: string, change: Promise<unknown>) => {
    setHeld((h) => new Map(h).set(id, name));
    change.catch(() =>
      setHeld((h) => {
        if (h.get(id) !== name) return h;
        const next = new Map(h);
        next.delete(id);
        return next;
      }),
    );
  }, []);

  // Once a load has the name, or the thing is gone, there's nothing left to hold.
  useEffect(() => {
    if (!loaded || held.size === 0) return;
    const items = [...loaded.projects, ...loaded.labels, ...loaded.filters];
    const next = new Map(held);
    for (const [id, name] of held) {
      const item = items.find((i) => i.id === id);
      if (!item || item.name === name) next.delete(id);
    }
    if (next.size !== held.size) setHeld(next);
  }, [loaded, held]);

  const page = useMemo(
    () =>
      loaded && held.size > 0
        ? { ...loaded, projects: named(loaded.projects, held), filters: named(loaded.filters, held) }
        : loaded,
    [loaded, held],
  );
  const sidebar = useMemo(
    () => (page && held.size > 0 ? { ...page, labels: named(page.labels, held) } : page),
    [page, held],
  );
  return { page, sidebar, holdName };
}
