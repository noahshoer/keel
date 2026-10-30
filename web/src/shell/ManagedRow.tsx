// A project or label row in the sidebar with hover (or long-press) actions:
// rename inline, delete after an inline confirm (crates/ui/src/sidebar.rs
// `ManagedRow`), without the glass menu and prompt.
import { type ReactNode, useRef, useState } from "react";
import { openAndFocus } from "./focus";
import { PencilIcon, TrashSmallIcon } from "./icons";

type Mode = "normal" | "renaming" | "confirm";

export function ManagedRow({
  name,
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
      <form
        className="project-form"
        onSubmit={(e) => {
          e.preventDefault();
          const n = draft.trim();
          if (n) onRename(n);
          setMode("normal");
        }}
      >
        <input
          ref={input}
          className="project-input"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") setMode("normal");
          }}
        />
      </form>
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
