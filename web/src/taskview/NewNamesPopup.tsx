// The save prompt for a retyped title that names new projects or labels,
// hung from the task view's title line like the streak's move prompt.
import { type RefObject, useRef } from "react";
import { createPortal } from "react-dom";
import type { NewNames } from "../bindings";
import { NewNamesPrompt } from "../editor/NewNamesPrompt";
import { useAnchoredPopup } from "../shell/useAnchoredPopup";

export function NewNamesPopup({
  anchor,
  names,
  onBack,
  onClose,
  onSave,
}: {
  anchor: RefObject<HTMLElement | null>;
  names: NewNames;
  /** "Back to editing": the typed title stays, unsaved. */
  onBack: () => void;
  /** A tap outside, Escape or a scroll; must keep its identity. */
  onClose: () => void;
  onSave: (unticked: NewNames) => void;
}) {
  const popup = useRef<HTMLDivElement>(null);
  useAnchoredPopup({ anchor, popup, close: onClose });
  return createPortal(
    <div ref={popup} className="popover nn-popover" role="dialog" aria-label="Create these with the task?">
      <NewNamesPrompt names={names} onBack={onBack} onSave={onSave} />
    </div>,
    document.body,
  );
}
