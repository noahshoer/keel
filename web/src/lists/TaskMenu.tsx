// What a long press (or a right click) on a task opens: add a sub-task, its
// streak when it repeats, and a red Delete. It opens under the row, or above
// it near the bottom; the row is lifted out of the dimmed page (`.lifted`).
import { type RefObject, useRef } from "react";
import { createPortal } from "react-dom";
import { ArrowRightIcon, FireIcon, SubtaskIcon, TrashSmallIcon } from "../shell/icons";
import { useAnchoredPopup } from "../shell/useAnchoredPopup";

export function TaskMenu({
  anchor,
  recurring,
  onAddSubtask,
  onStreak,
  onMove,
  onDelete,
  close,
}: {
  anchor: RefObject<HTMLElement | null>;
  /** Streak is for repeating tasks. */
  recurring: boolean;
  onAddSubtask: () => void;
  onStreak: () => void;
  /** Opens the Move to… list. */
  onMove: (() => void) | undefined;
  onDelete: () => void;
  /** Must keep its identity. */
  close: () => void;
}) {
  const popup = useRef<HTMLDivElement>(null);
  useAnchoredPopup({ anchor, popup, close });
  const choose = (act: () => void) => () => {
    close();
    act();
  };
  return createPortal(
    // biome-ignore lint/a11y/useKeyWithClickEvents: only keeps a click in the portal from reaching the row.
    // biome-ignore lint/a11y/noStaticElementInteractions: see above.
    <div ref={popup} className="popover menu row-menu task-menu" onClick={(e) => e.stopPropagation()}>
      <button type="button" className="menu-item" onClick={choose(onAddSubtask)}>
        <span className="menu-icon">
          <SubtaskIcon />
        </span>
        <span className="menu-label">Add sub-task</span>
      </button>
      {recurring ? (
        <button type="button" className="menu-item" onClick={choose(onStreak)}>
          <span className="menu-icon">
            <FireIcon />
          </span>
          <span className="menu-label">Streak</span>
        </button>
      ) : null}
      {onMove ? (
        <button type="button" className="menu-item" onClick={choose(onMove)}>
          <span className="menu-icon">
            <ArrowRightIcon />
          </span>
          <span className="menu-label">Move to…</span>
        </button>
      ) : null}
      <button type="button" className="menu-item danger" onClick={choose(onDelete)}>
        <span className="menu-icon">
          <TrashSmallIcon />
        </span>
        <span className="menu-label">Delete</span>
      </button>
    </div>,
    document.body,
  );
}
