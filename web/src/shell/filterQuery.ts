// A filter's query while it's edited: the hint, and the backend's live check.
import { useEffect, useRef, useState } from "react";
import { invoke } from "../ipc/invoke";

export const FILTER_HINT =
  "today · overdue · 7 days · p1 · #Project · /Section · @label · no date · search: word · due before: +3 days · deadline before: +7 days · no deadline — combine with & | ! ( ) and , for separate lists";

/** A query being edited, and the backend's message about it as you type. */
export function useQueryCheck(initialQuery: string): {
  query: string;
  setQuery: (q: string) => void;
  problem: string | null;
} {
  const [query, setQuery] = useState(initialQuery);
  // null = valid (or empty); a string = the backend's message.
  const [problem, setProblem] = useState<string | null>(null);
  // Bumped per keystroke so a slow, stale check can't overwrite a newer one.
  const generation = useRef(0);
  const check = (q: string) => {
    const mine = ++generation.current;
    const result = q.trim() ? invoke("check_filter", { query: q }) : Promise.resolve(null);
    result.then(
      () => {
        if (generation.current === mine) setProblem(null);
      },
      (e: unknown) => {
        if (generation.current === mine) setProblem(String(e));
      },
    );
  };
  // Validate the initial query once (edit mode).
  const first = useRef(initialQuery);
  // biome-ignore lint/correctness/useExhaustiveDependencies: runs once, for the query the form opened with.
  useEffect(() => check(first.current), []);
  return {
    query,
    setQuery: (q) => {
      setQuery(q);
      check(q);
    },
    problem,
  };
}
