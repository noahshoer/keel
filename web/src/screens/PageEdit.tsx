// Editing a project, label or filter on its page: the title itself is the field,
// and a filter's query takes the line under it. Enter saves; Escape or a click
// outside leaves things as they were, as in the sidebar's forms.
import { type RefObject, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { holdPage } from "../app/pageHold";
import type { Filter } from "../bindings";
import { useCloseOnClickOutside } from "../shell/clickOutside";
import { FilterField } from "../shell/FilterField";
import { type FilterNames, useQueryCheck } from "../shell/filterQuery";
import type { PageEditing } from "../shell/listActions";

function TitleField({
  form,
  field,
  value,
  label,
  onChange,
  onSubmit,
  onClose,
}: {
  form: RefObject<HTMLFormElement | null>;
  field: RefObject<HTMLInputElement | null>;
  value: string;
  label: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onClose: () => void;
}) {
  return (
    <form
      ref={form}
      className="view-title view-title-edit"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
    >
      <input
        ref={field}
        aria-label={label}
        enterKeyHint="done"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") onClose();
        }}
      />
    </form>
  );
}

function NameEdit({
  initial,
  field,
  onSave,
  onClose,
}: {
  initial: string;
  field: RefObject<HTMLInputElement | null>;
  onSave: (name: string) => void;
  onClose: () => void;
}) {
  const [name, setName] = useState(initial);
  const form = useRef<HTMLFormElement>(null);
  useCloseOnClickOutside(form, onClose);
  return (
    <TitleField
      form={form}
      field={field}
      value={name}
      label="Name"
      onChange={setName}
      onSubmit={() => {
        const n = name.trim();
        onClose();
        if (n && n !== initial) onSave(n);
      }}
      onClose={onClose}
    />
  );
}

function FilterEdit({
  filter,
  field,
  querySlot,
  names,
  onSave,
  onClose,
}: {
  filter: Filter;
  names: FilterNames;
  field: RefObject<HTMLInputElement | null>;
  querySlot: HTMLElement | null;
  onSave: (name: string, query: string) => void;
  onClose: () => void;
}) {
  const [name, setName] = useState(filter.name);
  const check = useQueryCheck(filter.query, names);
  const { query, problem } = check;
  const titleForm = useRef<HTMLFormElement>(null);
  const queryForm = useRef<HTMLFormElement>(null);
  useCloseOnClickOutside([titleForm, queryForm], onClose);
  const canSave = name.trim() !== "" && query.trim() !== "" && problem === null;
  const submit = () => {
    if (!canSave) return;
    void check.confirm().then((ok) => {
      if (!ok) return;
      onClose();
      onSave(name.trim(), query.trim());
    });
  };
  return (
    <>
      <TitleField
        form={titleForm}
        field={field}
        value={name}
        label="Filter name"
        onChange={setName}
        onSubmit={submit}
        onClose={onClose}
      />
      {querySlot
        ? createPortal(
            <form
              ref={queryForm}
              className="filter-form"
              onSubmit={(e) => {
                e.preventDefault();
                submit();
              }}
            >
              <FilterField
                check={check}
                labels={names.labels}
                label="Query"
                onSubmit={submit}
                onEscape={onClose}
              />
              <div className="confirm-actions">
                <button type="button" className="btn btn-cancel" onClick={onClose}>
                  Cancel
                </button>
                <button type="submit" className="btn btn-add" disabled={!canSave}>
                  Save
                </button>
              </div>
            </form>,
            querySlot,
          )
        : null}
    </>
  );
}

export function PageEdit({
  editing,
  field,
  querySlot,
  names,
  onClose,
}: {
  editing: PageEditing;
  names: FilterNames;
  field: RefObject<HTMLInputElement | null>;
  /** Where a filter's query goes: the line under the title. */
  querySlot: HTMLElement | null;
  onClose: () => void;
}) {
  // WebKit would otherwise shift the page to reveal the field.
  useEffect(() => holdPage(), []);
  return editing.kind === "name" ? (
    <NameEdit initial={editing.name} field={field} onSave={editing.rename} onClose={onClose} />
  ) : (
    <FilterEdit
      filter={editing.filter}
      field={field}
      querySlot={querySlot}
      names={names}
      onSave={editing.save}
      onClose={onClose}
    />
  );
}
