// The collapsed "+ Add task" row; a tap opens a quick-add editor that stays
// open after each add (crates/ui/src/editor.rs `AddTask`, no-glass path).
// The editor's title is focused inside the tap, so iOS raises the keyboard.
import { useRef, useState } from "react";
import { flushSync } from "react-dom";
import type { EditorValues, Label, Project, Section } from "../bindings";
import { localToday } from "../clock";
import { PlusIcon } from "../shell/icons";
import { addFromEditor } from "./save";
import { TaskEditor } from "./TaskEditor";

export function AddTask({
  defaults,
  projects,
  sections,
  labels,
  main = false,
  onChanged,
}: {
  /** What a new task starts with (the view's project, date or label). */
  defaults: EditorValues;
  projects: Project[];
  sections: Section[];
  labels: Label[];
  /** The view's own row (`view-add`), as opposed to a section's. */
  main?: boolean;
  onChanged: () => void;
}) {
  const [open, setOpen] = useState(false);
  const title = useRef<HTMLTextAreaElement>(null);
  if (!open) {
    return (
      // biome-ignore lint/a11y/useKeyWithClickEvents: the Dioxus row is a plain div (app.css styles it).
      // biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI.
      <div
        className={main ? "add-task-row view-add" : "add-task-row"}
        onClick={() => {
          flushSync(() => setOpen(true));
          title.current?.focus();
        }}
      >
        <span className="plus">
          <PlusIcon size={14} />
        </span>
        Add task
      </div>
    );
  }
  return (
    <TaskEditor
      initial={defaults}
      projects={projects}
      sections={sections}
      labels={labels}
      keepLabel={null}
      existing={null}
      submitText="Add task"
      quickAdd
      keepOpen
      onSubmit={(v) => {
        addFromEditor(v, projects, sections, labels, localToday()).then(onChanged, onChanged);
      }}
      onCancel={() => setOpen(false)}
      onChanged={onChanged}
      titleRef={title}
    />
  );
}
