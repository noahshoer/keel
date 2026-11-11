// The page's filter panel: the query field (the same one the sidebar uses), how
// many tasks it matches, the saved filters to start from, and "Save as a
// filter". What's typed previews on the page behind at once (`onDraft`) and is
// kept when the panel closes, as long as it reads.
import { type RefObject, useCallback, useEffect, useRef, useState } from "react";
import { holdPage } from "../app/pageHold";
import type { Filter } from "../bindings";
import { addFilter } from "../ipc/actions";
import { FilterField } from "../shell/FilterField";
import { type FilterNames, useQueryCheck } from "../shell/filterQuery";
import { openAndFocus } from "../shell/focus";
import { ArrangePanel } from "./ArrangePanel";

export function FilterSheet({
  pageName,
  initial,
  names,
  filters,
  counts,
  anchor,
  fieldRef,
  onDraft,
  onSave,
  onFilterAdded,
  onClose,
}: {
  pageName: string;
  /** The saved filter. */
  initial: string;
  names: FilterNames;
  filters: Filter[];
  /** What the page shows of what it has, with the draft applied. */
  counts: { shown: number; total: number };
  /** The header button it opens from. */
  anchor: RefObject<HTMLElement | null>;
  /** For focusing the field inside the tap that opens the panel. */
  fieldRef?: RefObject<HTMLTextAreaElement | null>;
  /** A draft that reads (blank is no filter) to preview; null when it doesn't. */
  onDraft: (query: string | null) => void;
  /** A blank query clears the filter. */
  onSave: (query: string) => void;
  /** A filter was created from the draft. */
  onFilterAdded?: () => void;
  /** Must keep its identity. */
  onClose: () => void;
}) {
  const check = useQueryCheck(initial, names);
  const { query, problem } = check;
  const blank = query.trim() === "";
  const valid = blank || problem === null;
  const changed = query.trim() !== initial.trim();
  const draft = valid ? query.trim() : null;

  const latestDraft = useRef(onDraft);
  latestDraft.current = onDraft;
  const first = useRef(true);
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    latestDraft.current(draft);
  }, [draft]);
  useEffect(() => holdPage(), []);

  // However it closes, a draft that reads is kept, once.
  const done = useRef(false);
  const finish = () => {
    if (done.current) return;
    done.current = true;
    if (valid && changed) onSave(query.trim());
    onClose();
  };
  const latestFinish = useRef(finish);
  latestFinish.current = finish;
  const close = useCallback(() => latestFinish.current(), []);
  const submit = () => {
    if (!valid) return;
    if (blank) {
      finish();
      return;
    }
    void check.confirm().then((ok) => ok && finish());
  };
  const clear = () => {
    if (done.current) return;
    done.current = true;
    onSave("");
    onClose();
  };

  const [naming, setNaming] = useState(false);
  const [name, setName] = useState("");
  const nameRef = useRef<HTMLInputElement>(null);
  const saveable = !blank && problem === null && !filters.some((f) => f.query.trim() === query.trim());
  const create = () => {
    const n = name.trim();
    if (n === "" || !saveable) return;
    void addFilter(n, query.trim()).then(() => {
      setNaming(false);
      setName("");
      onFilterAdded?.();
    });
  };

  return (
    <ArrangePanel
      title={`Filter ${pageName}`}
      action={{ label: "Clear", run: clear }}
      anchor={anchor}
      onClose={close}
      layoutKey={`${problem !== null}|${filters.length}|${saveable}|${naming}`}
    >
      <div className="filter-panel">
        <FilterField
          check={check}
          labels={names.labels}
          label="Query"
          onSubmit={submit}
          onEscape={close}
          inputRef={fieldRef}
        />
        <p className="filter-count">{`${counts.shown} of ${counts.total} tasks match`}</p>
        {saveable && naming ? (
          <input
            ref={nameRef}
            className="project-input"
            placeholder="Filter name"
            aria-label="Filter name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                create();
              } else if (e.key === "Escape") {
                e.stopPropagation();
                setNaming(false);
              }
            }}
          />
        ) : saveable ? (
          <button
            type="button"
            className="filter-save-as"
            onClick={() => openAndFocus(() => setNaming(true), nameRef)}
          >
            Save as a filter
          </button>
        ) : null}
        {filters.length > 0 ? (
          <>
            <div className="sort-section">Your filters</div>
            <ul className="sort-list">
              {filters.map((f) => (
                <li key={f.id}>
                  <button
                    type="button"
                    className="sort-choice"
                    title={f.query}
                    onClick={() => check.setQuery(f.query)}
                  >
                    {f.name}
                  </button>
                </li>
              ))}
            </ul>
          </>
        ) : null}
      </div>
    </ArrangePanel>
  );
}
