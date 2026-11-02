// The ••• in a project, label or filter page's header. The same popup as a long
// press on its sidebar row, except that Edit happens in the popup: the row may be
// out of sight in the closed drawer.
import { useRef, useState } from "react";
import type { Loaded } from "../app/load";
import type { View } from "../app/view";
import { ActionPopup, type PopupStep } from "../shell/ActionPopup";
import { FilterForm } from "../shell/FilterForm";
import { openAndFocus } from "../shell/focus";
import { MoreIcon } from "../shell/icons";
import { filterActions, type ListContext, labelActions, projectActions } from "../shell/listActions";
import { RenameForm } from "../shell/ManagedRow";

export function ViewMenu({
  view,
  loaded,
  setView,
  onChanged,
}: {
  view: View;
  loaded: Loaded;
  setView: (v: View) => void;
  onChanged: () => void;
}) {
  const [step, setStep] = useState<PopupStep>("closed");
  const [draft, setDraft] = useState("");
  const button = useRef<HTMLButtonElement>(null);
  const field = useRef<HTMLInputElement>(null);
  const close = () => setStep("closed");
  const context: ListContext = { view, setView, settle: (p) => p.then(onChanged, onChanged) };

  const project = view.kind === "project" ? loaded.projects.find((p) => p.id === view.id) : undefined;
  const label =
    view.kind === "label"
      ? loaded.labels.find((l) => l.name.toLowerCase() === view.name.toLowerCase())
      : undefined;
  const filter = view.kind === "filter" ? loaded.filters.find((f) => f.id === view.id) : undefined;
  const named = project ?? label;
  const actions = project
    ? projectActions(project, context)
    : label
      ? labelActions(label, context)
      : filter
        ? filterActions(filter, context)
        : null;
  if (!actions) return null;

  return (
    <>
      <button
        ref={button}
        type="button"
        className="icon-btn view-more"
        title="More"
        aria-label="More"
        onClick={() => setStep((s) => (s === "closed" ? "menu" : "closed"))}
      >
        <MoreIcon />
      </button>
      {step !== "closed" ? (
        <ActionPopup
          anchor={button}
          toggle={button}
          step={step}
          setStep={setStep}
          warning={actions.warning}
          onEdit={() =>
            openAndFocus(() => {
              setDraft(named?.name ?? "");
              setStep("edit");
            }, field)
          }
          onDelete={actions.remove}
          editor={
            filter && "save" in actions ? (
              <FilterForm
                initialName={filter.name}
                initialQuery={filter.query}
                submitText="Save"
                nameRef={field}
                onSave={(name, query) => {
                  close();
                  actions.save(name, query);
                }}
                onCancel={close}
              />
            ) : "rename" in actions ? (
              <RenameForm
                input={field}
                draft={draft}
                setDraft={setDraft}
                onSave={(name) => {
                  close();
                  actions.rename(name);
                }}
                onClose={close}
              />
            ) : null
          }
        />
      ) : null}
    </>
  );
}
