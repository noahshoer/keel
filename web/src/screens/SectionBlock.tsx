// A project section: its header (rename / delete on hover), its cards, and
// "+ Add section" below the last one (crates/ui/src/main.rs `SectionBlock`,
// `AddSection`). Forms opened by a tap focus their field inside the tap, so
// iOS raises the keyboard.
import { type FormEvent, useRef, useState } from "react";
import { flushSync } from "react-dom";
import type { Loaded } from "../app/load";
import type { Id, Reminder, Section, Task } from "../bindings";
import { blankValues, cards } from "../core";
import { AddTask } from "../editor/AddTask";
import { addSection, deleteSection, renameSection } from "../ipc/actions";
import { CardList } from "../lists/CardList";
import { PencilIcon, TrashSmallIcon } from "../shell/icons";

type Mode = "normal" | "renaming" | "confirm";

export function SectionBlock({
  section,
  tasks,
  all,
  loaded,
  today,
  reminders,
  onChanged,
}: {
  section: Section;
  tasks: Task[];
  /** Every task, for the cards' sub-task counts. */
  all: Task[];
  /** The lists the section's "+ Add task" editor picks from. */
  loaded: Pick<Loaded, "projects" | "sections" | "labels">;
  today: string;
  reminders: Reminder[];
  onChanged: () => void;
}) {
  const [mode, setMode] = useState<Mode>("normal");
  const [draft, setDraft] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const list = cards(tasks, all, "TopLevel");
  // What the section shows: its top-level tasks (sub-tasks go with them).
  const count = list.length;

  const startRename = () => {
    flushSync(() => {
      setDraft(section.name);
      setMode("renaming");
    });
    input.current?.focus();
  };
  const submit = (e: FormEvent) => {
    e.preventDefault();
    const name = draft.trim();
    setMode("normal");
    if (name) renameSection(section.id, name).then(onChanged, onChanged);
  };

  return (
    <div className="section-block">
      {mode === "renaming" ? (
        <form className="section-rename" onSubmit={submit}>
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
      ) : mode === "confirm" ? (
        <div className="confirm-box">
          <div className="confirm-text">{`Delete section “${section.name}” and its ${count} ${count === 1 ? "task" : "tasks"}?`}</div>
          <div className="confirm-actions">
            <button type="button" className="btn btn-cancel" onClick={() => setMode("normal")}>
              Cancel
            </button>
            <button
              type="button"
              className="btn btn-add"
              onClick={() => {
                setMode("normal");
                deleteSection(section.id).then(onChanged, onChanged);
              }}
            >
              Delete
            </button>
          </div>
        </div>
      ) : (
        <div className="section-title section-row">
          {section.name}
          <span className="muted">{count}</span>
          <span className="section-actions">
            <button type="button" className="icon-btn" title="Rename section" onClick={startRename}>
              <PencilIcon />
            </button>
            <button
              type="button"
              className="icon-btn"
              title="Delete section"
              onClick={() => setMode("confirm")}
            >
              <TrashSmallIcon />
            </button>
          </span>
        </div>
      )}
      <CardList cards={list} today={today} locations={null} reminders={reminders} onChanged={onChanged} />
      <AddTask
        defaults={blankValues(section.project_id, section.id, "", [])}
        projects={loaded.projects}
        sections={loaded.sections}
        labels={loaded.labels}
        onChanged={onChanged}
      />
    </div>
  );
}

/** "+ Add section" at the bottom of a project. */
export function AddSection({ projectId, onChanged }: { projectId: Id; onChanged: () => void }) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const input = useRef<HTMLInputElement>(null);
  if (!open) {
    return (
      // biome-ignore lint/a11y/useKeyWithClickEvents: the Dioxus divider is a plain div; keyboard users have the section forms.
      // biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI (app.css styles it).
      <div
        className="add-section-row"
        onClick={() => {
          flushSync(() => setOpen(true));
          input.current?.focus();
        }}
      >
        <span className="add-section-line" />
        <span className="add-section-label">Add section</span>
        <span className="add-section-line" />
      </div>
    );
  }
  return (
    <form
      className="section-rename"
      onSubmit={(e) => {
        e.preventDefault();
        const n = name.trim();
        if (!n) return;
        addSection(projectId, n).then(() => {
          setName("");
          setOpen(false);
          onChanged();
        }, onChanged);
      }}
    >
      <input
        ref={input}
        className="project-input"
        placeholder="Name this section, then Enter"
        value={name}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") setOpen(false);
        }}
      />
    </form>
  );
}
