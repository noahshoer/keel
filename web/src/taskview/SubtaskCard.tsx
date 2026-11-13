// The task view's sub-tasks: a head with their progress and "Show completed",
// then a card of the direct sub-tasks and "Add sub-task" or its quick-add
// editor, which stays open after each add.
import { useMemo, useRef } from "react";
import { flushSync } from "react-dom";
import type { Loaded } from "../app/load";
import type { EditorValues, Id, TaskDetail } from "../bindings";
import { localToday } from "../clock";
import { blankValues } from "../core";
import { addFromEditor } from "../editor/save";
import { TaskEditor } from "../editor/TaskEditor";
import { SubtaskProgress } from "../lists/SubtaskProgress";
import { PlusIcon } from "../shell/icons";
import { useTaskNav } from "./nav-context";
import { SubtaskRow } from "./SubtaskRow";
import { settle } from "./view-rules";

export function SubtaskCard({
  detail,
  loaded,
  adding,
  setAdding,
  onOpen,
  onChanged,
}: {
  detail: TaskDetail;
  loaded: Loaded;
  /** The "Add sub-task" editor is showing (the view owns it: a list row's "Add sub-task" opens it too). */
  adding: boolean;
  setAdding: (on: boolean) => void;
  /** Opens a sub-task's view on top (saving the drafts first). */
  onOpen: (id: Id) => void;
  onChanged: () => void;
}) {
  const nav = useTaskNav();
  const { projects, sections, labels } = loaded;
  const { task, progress } = detail;
  const title = useRef<HTMLTextAreaElement>(null);
  const defaults = useMemo<EditorValues>(
    () => ({ ...blankValues(task.project_id, task.section_id, "", []), parent_id: task.id }),
    [task.project_id, task.section_id, task.id],
  );
  return (
    <div className="tv-section">
      <div className="tv-sub-head">
        <b>Sub-tasks</b>
        {progress !== null ? (
          <span className="tv-sub-side">
            <SubtaskProgress progress={progress} />
            {" · "}
            <button
              type="button"
              className="tv-toggle"
              onClick={() => nav.setShowCompleted(!nav.showCompleted)}
            >
              {nav.showCompleted ? "Hide completed" : "Show completed"}
            </button>
          </span>
        ) : null}
      </div>
      <div className="tv-card tv-subtasks">
        {detail.visible.map((row) => (
          <SubtaskRow key={row.task.id} row={row} onOpen={onOpen} onChanged={onChanged} />
        ))}
        {adding ? (
          <div className="tv-add-editor">
            <TaskEditor
              initial={defaults}
              projects={projects}
              sections={sections}
              labels={labels}
              submitText="Add sub-task"
              onSubmit={(v) => {
                void settle(addFromEditor(v, projects, sections, labels, localToday())).then(onChanged);
              }}
              onCancel={() => setAdding(false)}
              onChanged={onChanged}
              titleRef={title}
            />
          </div>
        ) : (
          <button
            type="button"
            className="tv-add-sub"
            // The editor's title is focused inside the tap, so iOS raises the keyboard.
            onClick={() => {
              flushSync(() => setAdding(true));
              title.current?.focus();
            }}
          >
            <PlusIcon size={14} />
            Add sub-task
          </button>
        )}
      </div>
    </div>
  );
}
