// Saved filters in the sidebar (crates/ui/src/filters.rs): the add / edit
// form with live query validation, and a filter row (open, edit, delete).
import { type RefObject, useEffect, useRef, useState } from "react";
import type { Filter } from "../bindings";
import { invoke } from "../ipc/invoke";
import { openAndFocus } from "./focus";
import { FunnelIcon, PencilIcon, TrashSmallIcon } from "./icons";

const HINT =
  "today · overdue · 7 days · p1 · #Project · /Section · @label · no date · search: word · due before: +3 days · deadline before: +7 days · no deadline — combine with & | ! ( ) and , for separate lists";

/**
 * Name + query. Save is enabled only for a non-empty name and a query the
 * backend accepts; problems show as you type. `nameRef` is focused by the
 * caller inside the tap that opened the form.
 */
export function FilterForm({
  initialName,
  initialQuery,
  submitText,
  nameRef,
  onSave,
  onCancel,
}: {
  initialName: string;
  initialQuery: string;
  submitText: string;
  nameRef?: RefObject<HTMLInputElement | null>;
  onSave: (name: string, query: string) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(initialName);
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
  const canSave = name.trim() !== "" && query.trim() !== "" && problem === null;

  return (
    <form
      className="filter-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (canSave) onSave(name.trim(), query.trim());
      }}
    >
      <input
        ref={nameRef}
        className="project-input"
        placeholder="Filter name"
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
      <input
        className={problem !== null ? "project-input mono invalid" : "project-input mono"}
        placeholder="Query, e.g. today & p1"
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
          check(e.target.value);
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") onCancel();
        }}
      />
      {problem !== null ? (
        <div className="filter-problem">{problem}</div>
      ) : (
        <div className="filter-hint">{HINT}</div>
      )}
      <div className="confirm-actions">
        <button type="button" className="btn btn-cancel" onClick={onCancel}>
          Cancel
        </button>
        <button type="submit" className="btn btn-add" disabled={!canSave}>
          {submitText}
        </button>
      </div>
    </form>
  );
}

type Mode = "normal" | "editing" | "confirm";

export function FilterRow({
  filter,
  active,
  onOpen,
  onSave,
  onDelete,
}: {
  filter: Filter;
  active: boolean;
  onOpen: () => void;
  onSave: (name: string, query: string) => void;
  onDelete: () => void;
}) {
  const [mode, setMode] = useState<Mode>("normal");
  const nameRef = useRef<HTMLInputElement>(null);
  if (mode === "editing") {
    return (
      <FilterForm
        initialName={filter.name}
        initialQuery={filter.query}
        submitText="Save"
        nameRef={nameRef}
        onSave={(n, q) => {
          setMode("normal");
          onSave(n, q);
        }}
        onCancel={() => setMode("normal")}
      />
    );
  }
  if (mode === "confirm") {
    return (
      <div className="confirm-box">
        <div className="confirm-text">{`Delete filter “${filter.name}”? Your tasks aren’t affected.`}</div>
        <div className="confirm-actions">
          <button type="button" className="btn btn-cancel" onClick={() => setMode("normal")}>
            Cancel
          </button>
          <button
            type="button"
            className="btn btn-add"
            onClick={() => {
              setMode("normal");
              onDelete();
            }}
          >
            Delete
          </button>
        </div>
      </div>
    );
  }
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: same markup as the Dioxus sidebar (div rows).
    // biome-ignore lint/a11y/noStaticElementInteractions: see above.
    <div
      className={active ? "nav-item active" : "nav-item"}
      data-menu-id={`filter:${filter.id}`}
      title={filter.query}
      onClick={onOpen}
    >
      <span className="nav-icon filter">
        <FunnelIcon />
      </span>
      <span className="nav-name">{filter.name}</span>
      <span className="nav-actions">
        <button
          type="button"
          className="icon-btn"
          title="Edit filter"
          onClick={(e) => {
            e.stopPropagation();
            openAndFocus(() => setMode("editing"), nameRef);
          }}
        >
          <PencilIcon />
        </button>
        <button
          type="button"
          className="icon-btn"
          title="Delete filter"
          onClick={(e) => {
            e.stopPropagation();
            setMode("confirm");
          }}
        >
          <TrashSmallIcon />
        </button>
      </span>
    </div>
  );
}
