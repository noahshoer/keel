// A completed task in the "Completed" section: the filled check reopens it
// (crates/ui/src/task_row.rs `CompletedRow`); the body opens the task view.
import type { Task } from "../bindings";
import { localDateOf } from "../clock";
import { dueLabel } from "../core";
import { deleteTask, uncompleteTask } from "../ipc/actions";
import { CheckIcon, TrashIcon } from "../shell/icons";
import { useTaskNav } from "../taskview/nav-context";

export function CompletedRow({
  task,
  today,
  location,
  onChanged,
}: {
  task: Task;
  today: string;
  location: string | null;
  onChanged: () => void;
}) {
  const nav = useTaskNav();
  const day = task.completed_at === null ? null : localDateOf(task.completed_at);
  const when = day === null ? "" : dueLabel(day, today);
  return (
    <div className="task completed">
      <div className="task-row">
        <button
          type="button"
          className="check done"
          title="Mark as not done"
          onClick={() => {
            uncompleteTask(task.id).then(onChanged, onChanged);
          }}
        >
          <CheckIcon />
        </button>
        {/* biome-ignore lint/a11y/useKeyWithClickEvents: same markup as the Dioxus UI. */}
        {/* biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI. */}
        <div className="task-body" title="Open task" onClick={() => nav.open(task.id)}>
          <div className="task-content">{task.content}</div>
          <div className="task-meta">
            <span className="done-when">{`Completed · ${when}`}</span>
          </div>
        </div>
        <div className="task-actions">
          <button
            type="button"
            className="icon-btn"
            title="Delete task"
            onClick={() => {
              deleteTask(task.id).then(onChanged, onChanged);
            }}
          >
            <TrashIcon />
          </button>
        </div>
        {location !== null ? <span className="task-project">{location}</span> : null}
      </div>
    </div>
  );
}
