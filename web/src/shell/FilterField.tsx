// A filter's query field: the terms keel-core recognizes are highlighted as on
// the task card, an unknown name is underlined red, and the line under it says
// what's wrong, with the fix as a button (or the hint while there's nothing to say).
import { useId } from "react";
import type { Label } from "../bindings";
import { HighlightedTitle } from "../editor/HighlightedTitle";
import { FILTER_HINT, type QueryCheck } from "./filterQuery";

export function FilterField({
  check,
  labels,
  label,
  onSubmit,
  onEscape,
}: {
  check: QueryCheck;
  labels: Label[];
  /** The field's accessible name. */
  label?: string;
  onSubmit: () => void;
  onEscape: () => void;
}) {
  const { query, setQuery, segments, problem, fix, unhighlight } = check;
  const problemId = useId();
  return (
    <>
      <HighlightedTitle
        className={problem !== null ? "filter-field mono invalid" : "filter-field mono"}
        segments={segments}
        labels={labels}
        value={query}
        placeholder="Query, e.g. today & p1"
        label={label}
        invalid={problem !== null}
        describedBy={problem !== null ? problemId : undefined}
        autoFocus={false}
        onInput={setQuery}
        onEnter={onSubmit}
        onEscape={onEscape}
        onUnhighlight={unhighlight}
      />
      {/* Always mounted, so the first problem is announced (a live region
          inserted with its content often isn't read). Only the problem goes
          in it; the long hint stays outside, so it isn't read on every fix. */}
      <div className="filter-live" aria-live="polite" aria-atomic="true">
        {problem !== null ? (
          <div className="filter-problem" id={problemId}>
            {problem}
            {fix !== null ? (
              <>
                {" "}
                <button type="button" className="filter-fix" onClick={() => setQuery(fix.query)}>
                  {fix.label}
                </button>
              </>
            ) : null}
          </div>
        ) : null}
      </div>
      {problem === null ? <div className="filter-hint">{FILTER_HINT}</div> : null}
    </>
  );
}
