// A filter's query while it's edited: the hint, and keel-core's live reading
// (highlighted terms, the first problem, a one-tap fix). No round trip; the
// backend's `check_filter` is the backstop when the query is saved.
import { useMemo, useRef, useState } from "react";
import type { Label, Project, Section, TitleSegment } from "../bindings";
import { localToday } from "../clock";
import { filterReading, filterUnhighlightAt } from "../core";
import { invoke } from "../ipc/invoke";

export const FILTER_HINT =
  'today · in 3 days · next friday · every monday · overdue · next 7 days · p1 · #Project · /Section · @label · no date · streak · "word" · due before: +3 days · deadline before: +7 days — combine with & | ! ( ) and , for separate lists';

/** What a query's names are checked against. */
export type FilterNames = { projects: Project[]; sections: Section[]; labels: Label[] };

export type QueryCheck = {
  query: string;
  setQuery: (q: string) => void;
  segments: TitleSegment[];
  problem: string | null;
  fix: { label: string; query: string } | null;
  /** Quotes the term ending at `caret`; whether there was one. */
  unhighlight: (caret: number) => boolean;
  /** The backend's last word on the query, for saving; `false` shows its message. A second call
   * while one is pending gives `false` and does nothing, so a double Enter saves once. */
  confirm: () => Promise<boolean>;
};

export function useQueryCheck(initialQuery: string, names: FilterNames): QueryCheck {
  const [query, setRaw] = useState(initialQuery);
  const [backend, setBackend] = useState<string | null>(null);
  const checking = useRef(false);
  const today = localToday();
  const { projects, sections, labels } = names;
  const reading = useMemo(
    () => filterReading(query, today, projects, sections, labels),
    [query, today, projects, sections, labels],
  );
  const setQuery = (q: string) => {
    setRaw(q);
    setBackend(null);
  };
  return {
    query,
    setQuery,
    segments: reading.segments,
    // An empty field has nothing to flag; Save just stays off.
    problem: query.trim() === "" ? null : (reading.problem ?? backend),
    fix: reading.fix,
    unhighlight: (caret) => {
      const next = filterUnhighlightAt(query, projects, sections, labels, today, caret);
      if (next === null) return false;
      setQuery(next);
      return true;
    },
    confirm: () => {
      if (checking.current) return Promise.resolve(false);
      checking.current = true;
      return invoke("check_filter", { query })
        .then(
          () => true,
          (e: unknown) => {
            setBackend(String(e));
            return false;
          },
        )
        .finally(() => {
          checking.current = false;
        });
    },
  };
}
