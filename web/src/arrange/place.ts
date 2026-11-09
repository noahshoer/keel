// Which saved arrangement belongs to the page being shown. Only the lookup is
// here; what an arrangement does to the tasks is keel-core's (`arrange`).

import type { Loaded } from "../app/load";
import type { View } from "../app/view";
import type { Arrangement, PlacedArrangement, ViewPlace } from "../bindings";

export const NO_ARRANGEMENT: Arrangement = { filter: "", groups: [], order: [] };

/** null for Search, and for a label page whose name has no label row. */
export function placeOf(view: View, loaded: Pick<Loaded, "inbox" | "labels">): ViewPlace | null {
  switch (view.kind) {
    case "inbox":
      return { kind: "project", id: loaded.inbox.id };
    case "today":
      return { kind: "today" };
    case "upcoming":
      return { kind: "upcoming" };
    case "project":
      return { kind: "project", id: view.id };
    case "filter":
      return { kind: "filter", id: view.id };
    case "label": {
      const name = view.name.toLowerCase();
      const label = loaded.labels.find((l) => !l.is_deleted && l.name.toLowerCase() === name);
      return label ? { kind: "label", id: label.id } : null;
    }
    case "search":
      return null;
  }
}

const samePlace = (a: ViewPlace, b: ViewPlace): boolean =>
  a.kind === b.kind && (a.kind === "today" || a.kind === "upcoming" || ("id" in b && a.id === b.id));

/** The page's saved arrangement, or `NO_ARRANGEMENT`. */
export function arrangementOf(place: ViewPlace | null, all: PlacedArrangement[]): Arrangement {
  if (!place) return NO_ARRANGEMENT;
  return all.find((p) => samePlace(p.place, place))?.arrangement ?? NO_ARRANGEMENT;
}

/** Whether the arrangement changes the page (a blank filter doesn't). */
export function isArranged(a: Arrangement): boolean {
  return a.filter.trim() !== "" || a.groups.length > 0 || a.order.length > 0;
}
