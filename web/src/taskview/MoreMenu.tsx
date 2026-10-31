// The task view's ••• menu without glass: Streak for a repeating task, then
// Delete (crates/ui/src/task_view/view.rs `view_actions` and its popover).
import { useRef } from "react";
import { usePlacedPopover } from "../editor/popover";

export function MoreMenu({
  recurring,
  onStreak,
  onDelete,
}: {
  recurring: boolean;
  onStreak: () => void;
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
      <button type="button" className="menu-item danger" onClick={onDelete}>
        <span className="menu-label">Delete task</span>
      </button>
    </div>
  );
}
