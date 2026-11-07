// Asks before a date pick moves one occurrence of a task with a streak going
// (issue #8): the move resets the streak. Hangs from the task view's fields,
// in the look of a row's Delete confirm (`ActionPopup`).
import { type RefObject, useRef } from "react";
import { createPortal } from "react-dom";
import { useAnchoredPopup } from "../shell/useAnchoredPopup";

export function MovePrompt({
  anchor,
  streak,
  onMove,
  onCancel,
}: {
  anchor: RefObject<HTMLElement | null>;
  /** The streak a move resets. */
  streak: number;
  onMove: () => void;
  /** Must keep its identity (it's the outside-click close). */
  onCancel: () => void;
}) {
  const popup = useRef<HTMLDivElement>(null);
  useAnchoredPopup({ anchor, popup, close: onCancel });
  return createPortal(
    <div
      ref={popup}
      className="popover menu row-menu move-prompt"
      role="alertdialog"
      aria-label="Move this one?"
    >
      <div className="confirm-text">Moving just this one resets its streak of {streak}. Your best stays.</div>
      <div className="confirm-actions">
        <button type="button" className="btn btn-cancel" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className="btn btn-danger" onClick={onMove}>
          Move
        </button>
      </div>
    </div>,
    document.body,
  );
}
