// A project or label row in the sidebar whose long press opens a menu: rename
// inline, delete after a confirm
// (crates/ui/src/sidebar.rs `ManagedRow`), without the glass menu and prompt.
import { type ReactNode, type RefObject, useRef, useState } from "react";
import { useCloseOnClickOutside } from "./clickOutside";
import { openAndFocus } from "./focus";
import { RowMenu } from "./RowMenu";

type Mode = "normal" | "renaming";

/** The row while it's renamed: Enter saves, Escape or a click outside leaves it as it was. */
export function RenameForm({
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
  warning,
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
  /** What Delete asks first. */
  warning: string;
  /** `project:<id>` / `label:<id>`. */
  menuId: string;
  onOpen: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
}) {
  const [mode, setMode] = useState<Mode>("normal");
  const [draft, setDraft] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const row = useRef<HTMLDivElement>(null);
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
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: same markup as the Dioxus sidebar (div rows).
    // biome-ignore lint/a11y/noStaticElementInteractions: see above.
    <div ref={row} className={active ? "nav-item active" : "nav-item"} data-menu-id={menuId} onClick={onOpen}>
      <span className={`nav-icon ${iconClass}`}>{icon}</span>
      <span className="nav-name">{name}</span>
      {count !== undefined && count > 0 ? <span className="count">{count}</span> : null}
      <RowMenu
        row={row}
        warning={warning}
        onEdit={() =>
          openAndFocus(() => {
            setDraft(name);
            setMode("renaming");
          }, input)
        }
        onDelete={onDelete}
      />
    </div>
  );
}
