// A project section: its header (rename / delete on hover), its cards, and
// "+ Add section" below the last one (crates/ui/src/main.rs `SectionBlock`,
// `AddSection`). Forms opened by a tap focus their field inside the tap, so
// iOS raises the keyboard.
import { type FormEvent, type MutableRefObject, useContext, useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import type { Loaded } from "../app/load";
import type { Id, Reminder, Section, Task } from "../bindings";
import { blankValues, cards } from "../core";
import { AddTask } from "../editor/AddTask";
import { addSection, deleteSection, renameSection } from "../ipc/actions";
import { CardList } from "../lists/CardList";
import { DropZone } from "../lists/DropZone";
import { locationsOf } from "../lists/rowLayout";
import { ShowSubtasksContext } from "../lists/showSubtasks";
import { useCloseOnClickOutside } from "../shell/clickOutside";
import { PlusIcon } from "../shell/icons";
import { RowMenu } from "../shell/RowMenu";
import { ViewMenu } from "./ViewMenu";

type Mode = "normal" | "renaming";

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
  const { show: showSubtasks } = useContext(ShowSubtasksContext);
  const list = cards(tasks, all, showSubtasks ? "Grouped" : "TopLevel", today);
  // What the section shows: its top-level tasks (sub-tasks go with them).
  const count = list.length;

  const row = useRef<HTMLDivElement>(null);
  const warning = `Delete section “${section.name}” and its ${count} ${count === 1 ? "task" : "tasks"}?`;
  const remove = () => {
    deleteSection(section.id).then(onChanged, onChanged);
  };
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
    <DropZone
      className="section-block"
      project={section.project_id}
      section={section.id}
      name={section.name}
      onChanged={onChanged}
    >
      {mode === "renaming" ? (
        <form className="section-name" onSubmit={submit}>
          <input
            ref={input}
            className="section-name-input"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") setMode("normal");
            }}
          />
        </form>
      ) : (
        <div ref={row} className="section-title section-row">
          {section.name}
          <span className="muted">{count}</span>
          <ViewMenu
            className="icon-btn section-more"
            label="Section actions"
            warning={warning}
            onEdit={startRename}
            onDelete={remove}
          />
          <RowMenu row={row} warning={warning} onEdit={startRename} onDelete={remove} />
        </div>
      )}
      <CardList
        cards={list}
        today={today}
        locations={locationsOf(all, loaded.projects, loaded.sections)}
        labels={loaded.labels}
        reminders={reminders}
        onChanged={onChanged}
      />
      <AddTask
        defaults={blankValues(section.project_id, section.id, "", [])}
        projects={loaded.projects}
        sections={loaded.sections}
        labels={loaded.labels}
        onChanged={onChanged}
      />
    </DropZone>
  );
}

/**
 * "+ Add section" at the bottom of a project, a row like "+ Add task". `opener`
 * is given the function that opens its name field (the project's ••• calls it).
 */
export function AddSection({
  projectId,
  onChanged,
  opener,
}: {
  projectId: Id;
  onChanged: () => void;
  opener?: MutableRefObject<(() => void) | null>;
}) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const form = useRef<HTMLFormElement>(null);
  // A click outside the field puts the row back, and drops what was typed.
  useCloseOnClickOutside(form, () => {
    setOpen(false);
    setName("");
  });
  // Inside the tap, so iOS raises the keyboard.
  const openField = () => {
    flushSync(() => setOpen(true));
    input.current?.focus();
    input.current?.scrollIntoView?.({ block: "nearest" });
  };
  const latest = useRef(openField);
  latest.current = openField;
  useEffect(() => {
    if (!opener) return;
    opener.current = () => latest.current();
    return () => {
      opener.current = null;
    };
  }, [opener]);
  if (!open) {
    return (
      // biome-ignore lint/a11y/useKeyWithClickEvents: the Dioxus divider is a plain div; keyboard users have the section forms.
      // biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI (app.css styles it).
      <div className="add-section-row" onClick={openField}>
        <span className="plus">
          <PlusIcon size={14} />
        </span>
        Add section
      </div>
    );
  }
  return (
    <form
      ref={form}
      className="section-name section-name-new"
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
        className="section-name-input"
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
