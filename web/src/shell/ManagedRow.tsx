// A project or label row in the sidebar with hover (or long-press) actions:
// rename inline, delete after an inline confirm (crates/ui/src/sidebar.rs
// `ManagedRow`), without the glass menu and prompt.
import { type ReactNode, type RefObject, useRef, useState } from "react";
import { useCloseOnClickOutside } from "./clickOutside";
import { openAndFocus } from "./focus";
import { PencilIcon, TrashSmallIcon } from "./icons";

type Mode = "normal" | "renaming" | "confirm";

/** The row while it's renamed: Enter saves, Escape or a click outside leaves it as it was. */
function RenameForm({
  input,
  draft,
  setDraft,
  onSave,
  onClose,
}: {
  input: RefObject<HTMLInputElement | null>;
  draft: string;
  setDraft: (draft: string) => void;
  onSave: (name: string) => void;
  onClose: () => void;
}) {
  const form = useRef<HTMLFormElement>(null);
  useCloseOnClickOutside(form, onClose);
  return (
    <form
      ref={form}
      className="project-form"
      onSubmit={(e) => {
        e.preventDefault();
        const n = draft.trim();
        if (n) onSave(n);
        else onClose();
      }}
    >
      <input
        ref={input}
        className="project-input"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") onClose();
        }}
      />
    </form>
  );
}

export function ManagedRow({
  name,
  count,
  icon,
  iconClass,
  active,
  kind,
  menuId,
  onOpen,
  onRename,
  onDelete,
}: {
  name: string;
  /** The number beside its name, when it has one. */
  count?: number;
  icon: ReactNode;
  iconClass: string;
  active: boolean;
  kind: "project" | "label";
  /** `project:<id>` / `label:<id>`. */
  menuId: string;
  onOpen: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
}) {
  const [mode, setMode] = useState<Mode>("normal");
  const [draft, setDraft] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const warning =
    kind === "project"
      ? `Delete “${name}” and all its tasks?`
      : `Delete label “${name}”? It’s removed from its tasks.`;

  if (mode === "renaming") {
    return (
      <RenameForm
        input={input}
        draft={draft}
        setDraft={setDraft}
        onSave={(n) => {
          onRename(n);
          setMode("normal");
        }}
        onClose={() => setMode("normal")}
      />
    );
  }
  if (mode === "confirm") {
    return (
      <div className="confirm-box">
        <div className="confirm-text">{warning}</div>
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
    <div className={active ? "nav-item active" : "nav-item"} data-menu-id={menuId} onClick={onOpen}>
      <span className={`nav-icon ${iconClass}`}>{icon}</span>
      <span className="nav-name">{name}</span>
      {count !== undefined && count > 0 ? <span className="count">{count}</span> : null}
      <span className="nav-actions">
        <button
          type="button"
          className="icon-btn"
          title="Rename"
          onClick={(e) => {
            e.stopPropagation();
            openAndFocus(() => {
              setDraft(name);
              setMode("renaming");
            }, input);
          }}
        >
          <PencilIcon />
        </button>
        <button
          type="button"
          className="icon-btn"
          title="Delete"
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
