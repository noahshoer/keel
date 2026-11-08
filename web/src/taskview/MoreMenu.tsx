// The task view's ••• menu without glass: Streak for a repeating task, "Done
// with sub-tasks" for one with sub-tasks, then Delete
// (crates/ui/src/task_view/view.rs `view_actions` and its popover).
import { useRef } from "react";
import { usePlacedPopover } from "../editor/popover";
import { CheckCircleIcon } from "../shell/icons";

export function MoreMenu({
  recurring,
  onStreak,
  doneWith,
  onDoneWith,
  onDelete,
}: {
  recurring: boolean;
  onStreak: () => void;
  /** Whether "Done with sub-tasks" is on; `null` hides it. */
  doneWith: boolean | null;
  onDoneWith: () => void;
  onDelete: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  return (
    <div ref={ref} className="popover menu">
      {recurring ? (
        <button type="button" className="menu-item" onClick={onStreak}>
          <span className="menu-label">Streak</span>
        </button>
      ) : null}
      {doneWith !== null ? (
        <button type="button" className={doneWith ? "menu-item checked" : "menu-item"} onClick={onDoneWith}>
          <span className="menu-label">Done with sub-tasks</span>
          {doneWith ? (
            <span className="menu-check">
              <CheckCircleIcon />
            </span>
          ) : null}
        </button>
      ) : null}
      <button type="button" className="menu-item danger" onClick={onDelete}>
        <span className="menu-label">Delete task</span>
      </button>
    </div>
  );
}
