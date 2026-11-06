// One direct sub-task in the task view (crates/ui/src/task_view/view.rs
// `SubtaskRow`): its circle (complete, or reopen when checked), its name, its
// own "x/y", and a ›. The rest of the row opens its view.
import type { Id, SubtaskLine } from "../bindings";
import { completeTask, uncompleteTask } from "../ipc/actions";
import { CHECK_CLASS } from "../lists/chips";
import { SubtaskProgress } from "../lists/SubtaskProgress";
import { haptic } from "../shell/haptics";
import { CheckIcon, ChevronRightIcon } from "../shell/icons";
import { useTaskNav } from "./nav-context";

export function SubtaskRow({
  row,
  onOpen,
  onChanged,
}: {
  row: SubtaskLine;
  onOpen: (id: Id) => void;
  onChanged: () => void;
}) {
  const nav = useTaskNav();
  const t = row.task;
  const done = row.done;
  return (
    <div className={done ? "tv-sub done" : "tv-sub"}>
      <button
        type="button"
        className={done ? "check done" : CHECK_CLASS[t.priority]}
        title={done ? "Mark as not done" : "Complete sub-task"}
        onClick={() => {
          if (!nav.holdCircles()) return;
          if (!done) haptic("complete");
          // Not tied to the row: it goes once the task is done.
          const sent = done ? uncompleteTask(t.id) : completeTask(t.id);
          void sent.then(onChanged, onChanged).then(() => nav.releaseCircles());
        }}
      >
        <CheckIcon />
      </button>
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: the Dioxus row body is a plain div (app.css styles it). */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI. */}
      <div className="tv-sub-body" title="Open task" onClick={() => onOpen(t.id)}>
        <span className="task-content">{t.content}</span>
        {row.progress !== null ? <SubtaskProgress progress={row.progress} /> : null}
        <span className="card-chev">
          <ChevronRightIcon />
        </span>
      </div>
    </div>
  );
}
