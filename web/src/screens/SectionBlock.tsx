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
import { TASK_DROP } from "../gestures/drag";
import { addSection, addSectionGetId, deleteSection, moveTask, renameSection } from "../ipc/actions";
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
  shape,
  loaded,
  today,
  reminders,
  onChanged,
}: {
  section: Section;
  tasks: Task[];
  /** Every task, for the cards' sub-task counts. */
  all: Task[];
  /** The page's arrangement, when it has one: filters and orders the section's tasks. */
  shape?: ((tasks: Task[]) => Task[]) | null;
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
  const cardMode = showSubtasks ? "Grouped" : "TopLevel";
  const list = cards(shape ? shape(tasks) : tasks, all, cardMode, today);
  // What the section shows: its top-level tasks (sub-tasks go with them).
  const count = list.length;
  // Deleting removes everything in the section, whatever the page's filter hides.
  const total = shape ? cards(tasks, all, cardMode, today).length : count;

  const row = useRef<HTMLDivElement>(null);
  const warning = `Delete section “${section.name}” and its ${total} ${total === 1 ? "task" : "tasks"}?`;
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
  // A task dropped on the row waits for its section to be named, then moves into it.
  const [waiting, setWaiting] = useState<Waiting | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const form = useRef<HTMLFormElement>(null);
  const row = useRef<HTMLDivElement>(null);
  // A click outside the field puts the row back, and drops what was typed.
  useCloseOnClickOutside(form, () => {
    setOpen(false);
    setName("");
    setWaiting(null);
  });
  // Inside the tap, so iOS raises the keyboard.
  const openField = () => {
    flushSync(() => setOpen(true));
    input.current?.focus();
    input.current?.scrollIntoView?.({ block: "nearest" });
  };
  const latest = useRef(openField);
  latest.current = openField;
  // The row is where a dragged task starts a new section: letting go opens the name field.
  // biome-ignore lint/correctness/useExhaustiveDependencies: the row is a new element each time it comes back, so `open` is what it listens for.
  useEffect(() => {
    const el = row.current;
    if (!el) return;
    const dropped = (e: Event) => {
      const id = (e as CustomEvent<{ taskId: string }>).detail.taskId;
      setWaiting({ id, title: taskRow(id)?.querySelector(".task-content")?.textContent ?? "" });
      latest.current();
    };
    el.addEventListener(TASK_DROP, dropped);
    return () => el.removeEventListener(TASK_DROP, dropped);
  }, [open]);
  // The waiting task's own row is marked (an attribute: its component owns its `className`).
  const waitingId = waiting?.id ?? null;
  useEffect(() => {
    if (waitingId === null) return;
    const el = taskRow(waitingId);
    el?.setAttribute("data-move-waiting", "");
    return () => el?.removeAttribute("data-move-waiting");
  }, [waitingId]);
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
      <div
        ref={row}
        className="add-section-row"
        data-drop-zone=""
        data-drop-new=""
        data-drop-project={projectId}
        onClick={openField}
      >
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
        const task = waiting?.id ?? null;
        const made =
          task === null
            ? addSection(projectId, n)
            : addSectionGetId(projectId, n).then((id) => moveTask(task, projectId, id));
        made.then(() => {
          setName("");
          setOpen(false);
          setWaiting(null);
          onChanged();
        }, onChanged);
      }}
    >
      <input
        ref={input}
        className="section-name-input"
        placeholder={waiting === null ? "Name this section, then Enter" : "Name the new section, then Enter"}
        value={name}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setOpen(false);
            setWaiting(null);
          }
        }}
      />
      {waiting !== null ? (
        <div className="move-preview">
          <span className="move-preview-check" />
          <div className="move-preview-body">
            <div className="move-preview-title">{waiting.title}</div>
            <div className="move-preview-note">Moves here</div>
          </div>
        </div>
      ) : null}
    </form>
  );
}

/** A task dropped on Add section: its id, and its title for the copy under the field. */
type Waiting = { id: Id; title: string };

const taskRow = (id: Id) => document.querySelector(`.task[data-menu-id="${CSS.escape(id)}"]`);
