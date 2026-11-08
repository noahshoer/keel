// Saved filters in the sidebar (crates/ui/src/filters.rs): the add / edit
// form with live query validation, and a filter row (open, edit, delete).
import { type RefObject, useRef, useState } from "react";
import type { Filter } from "../bindings";
import { useCloseOnClickOutside } from "./clickOutside";
import { FilterField } from "./FilterField";
import { type FilterNames, useQueryCheck } from "./filterQuery";
import { openAndFocus } from "./focus";
import { FunnelIcon } from "./icons";
import { RowMenu } from "./RowMenu";

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
  names,
  onSave,
  onCancel,
}: {
  initialName: string;
  initialQuery: string;
  submitText: string;
  nameRef?: RefObject<HTMLInputElement | null>;
  names: FilterNames;
  onSave: (name: string, query: string) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(initialName);
  const check = useQueryCheck(initialQuery, names);
  const { query, problem } = check;
  const canSave = name.trim() !== "" && query.trim() !== "" && problem === null;
  const submit = () => {
    if (canSave) void check.confirm().then((ok) => ok && onSave(name.trim(), query.trim()));
  };
  const form = useRef<HTMLFormElement>(null);
  useCloseOnClickOutside(form, onCancel);

  return (
    <form
      ref={form}
      className="filter-form"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      <input
        ref={nameRef}
        className="project-input"
        placeholder="Filter name"
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
      <FilterField check={check} labels={names.labels} label="Query" onSubmit={submit} onEscape={onCancel} />
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

type Mode = "normal" | "editing";

export function FilterRow({
  filter,
  active,
  warning,
  names,
  onOpen,
  onSave,
  onDelete,
}: {
  filter: Filter;
  active: boolean;
  /** What Delete asks first. */
  warning: string;
  names: FilterNames;
  onOpen: () => void;
  onSave: (name: string, query: string) => void;
  onDelete: () => void;
}) {
  const [mode, setMode] = useState<Mode>("normal");
  const nameRef = useRef<HTMLInputElement>(null);
  const row = useRef<HTMLDivElement>(null);
  if (mode === "editing") {
    return (
      <FilterForm
        initialName={filter.name}
        initialQuery={filter.query}
        submitText="Save"
        nameRef={nameRef}
        names={names}
        onSave={(n, q) => {
          setMode("normal");
          onSave(n, q);
        }}
        onCancel={() => setMode("normal")}
      />
    );
  }
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: same markup as the Dioxus sidebar (div rows).
    // biome-ignore lint/a11y/noStaticElementInteractions: see above.
    <div
      ref={row}
      className={active ? "nav-item active" : "nav-item"}
      data-menu-id={`filter:${filter.id}`}
      title={filter.query}
      onClick={onOpen}
    >
      <span className="nav-icon filter">
        <FunnelIcon />
      </span>
      <span className="nav-name">{filter.name}</span>
      <RowMenu
        row={row}
        warning={warning}
        onEdit={() => openAndFocus(() => setMode("editing"), nameRef)}
        onDelete={onDelete}
      />
    </div>
  );
}
