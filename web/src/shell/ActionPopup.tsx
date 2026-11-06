// The popup of a project, label or filter: Edit, then a red Delete that asks
// first in the same place. It opens under what it belongs to, or above it near
// the bottom. Portaled, since a sidebar row scales while pressed and a fixed
// element inside it would move with it.
import { type ReactNode, type RefObject, useCallback, useRef } from "react";
import { createPortal } from "react-dom";
import { PencilIcon, TrashSmallIcon } from "./icons";
import { useAnchoredPopup } from "./useAnchoredPopup";

export type PopupStep = "closed" | "menu" | "confirm";

/** An action ahead of Edit and Delete, in the accent color (a project's "Add section"). */
export type PopupLead = { label: string; icon: ReactNode; run: () => void };

export function ActionPopup({
  anchor,
  toggle,
  lead,
  step,
  setStep,
  warning,
  onEdit,
  onDelete,
}: {
  /** What it opens against. */
  anchor: RefObject<HTMLElement | null>;
  /** The button that opens it, which closes it by itself. */
  toggle?: RefObject<HTMLElement | null>;
  lead?: PopupLead | undefined;
  step: Exclude<PopupStep, "closed">;
  setStep: (step: PopupStep) => void;
  warning: string;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const popup = useRef<HTMLDivElement>(null);

  const close = useCallback(() => setStep("closed"), [setStep]);
  useAnchoredPopup({ anchor, popup, toggle, close, layoutKey: step });

  return createPortal(
    // biome-ignore lint/a11y/useKeyWithClickEvents: only keeps a click in the portal from reaching the row.
    // biome-ignore lint/a11y/noStaticElementInteractions: see above.
    <div ref={popup} className="popover menu row-menu" onClick={(e) => e.stopPropagation()}>
      {step === "menu" ? (
        <>
          {lead ? (
            <button type="button" className="menu-item lead" onClick={lead.run}>
              <span className="menu-icon">{lead.icon}</span>
              <span className="menu-label">{lead.label}</span>
            </button>
          ) : null}
          <button type="button" className="menu-item" onClick={onEdit}>
            <span className="menu-icon">
              <PencilIcon />
            </span>
            <span className="menu-label">Edit</span>
          </button>
          <button type="button" className="menu-item danger" onClick={() => setStep("confirm")}>
            <span className="menu-icon">
              <TrashSmallIcon />
            </span>
            <span className="menu-label">Delete</span>
          </button>
        </>
      ) : null}
      {step === "confirm" ? (
        <>
          <div className="confirm-text">{warning}</div>
          <div className="confirm-actions">
            <button type="button" className="btn btn-cancel" onClick={() => setStep("closed")}>
              Cancel
            </button>
            <button
              type="button"
              className="btn btn-danger"
              onClick={() => {
                setStep("closed");
                onDelete();
              }}
            >
              Delete
            </button>
          </div>
        </>
      ) : null}
    </div>,
    document.body,
  );
}
