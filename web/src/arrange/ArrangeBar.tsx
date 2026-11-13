// The chips under a page's header that say what its arrangement does: the
// filter's query and the sort's summary, each with a ✕ that clears it. A page
// with no arrangement draws nothing at all.

import { useMemo } from "react";
import type { Arrangement } from "../bindings";
import { arrangementSummary } from "../core";
import { FunnelIcon, SortIcon, XSmallIcon } from "../shell/icons";
import { hasFilter, hasSort } from "./place";

export function ArrangeBar({
  arrangement,
  problem,
  onOpen,
  onClearFilter,
  onResetSort,
}: {
  arrangement: Arrangement;
  /** Why the saved filter can't run (keel-core's reading), if it can't. */
  problem: string | null;
  onOpen: (which: "filter" | "sort") => void;
  onClearFilter: () => void;
  onResetSort: () => void;
}) {
  const filtered = hasFilter(arrangement);
  const sorted = hasSort(arrangement);
  const summary = useMemo(() => (sorted ? arrangementSummary(arrangement) : ""), [sorted, arrangement]);
  if (!filtered && !sorted) return null;
  return (
    <div className="arrange-bar">
      {filtered ? (
        <span
          className={problem !== null ? "scope-chip arrange-chip problem" : "scope-chip arrange-chip"}
          title={problem ?? undefined}
        >
          <button type="button" className="arrange-chip-main" onClick={() => onOpen("filter")}>
            <FunnelIcon />
            <span className="arrange-chip-text">{arrangement.filter}</span>
          </button>
          <button type="button" className="arrange-chip-x" aria-label="Clear filter" onClick={onClearFilter}>
            <XSmallIcon />
          </button>
        </span>
      ) : null}
      {sorted ? (
        <span className="scope-chip arrange-chip">
          <button type="button" className="arrange-chip-main" onClick={() => onOpen("sort")}>
            <SortIcon />
            <span className="arrange-chip-text">{summary}</span>
          </button>
          <button type="button" className="arrange-chip-x" aria-label="Reset sort" onClick={onResetSort}>
            <XSmallIcon />
          </button>
        </span>
      ) : null}
    </div>
  );
}
