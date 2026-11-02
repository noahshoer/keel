// The popup of a project, label or filter: Edit, then a red Delete that asks
// first in the same place. It opens under what it belongs to, or above it near
// the bottom. Portaled, since a sidebar row scales while pressed and a fixed
// element inside it would move with it.
import { type RefObject, useEffect, useLayoutEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { PencilIcon, TrashSmallIcon } from "./icons";
import { placeRowMenu } from "./placeRowMenu";

export type PopupStep = "closed" | "menu" | "confirm";

export function ActionPopup({
  anchor,
  toggle,
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
  step: Exclude<PopupStep, "closed">;
  setStep: (step: PopupStep) => void;
  warning: string;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const popup = useRef<HTMLDivElement>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: each step is a different size.
  useLayoutEffect(() => {
    const at = anchor.current;
    const pop = popup.current;
    if (!at || !pop) return;
    const vh = window.visualViewport?.height ?? innerHeight;
    const place = placeRowMenu(at.getBoundingClientRect(), pop.getBoundingClientRect(), innerWidth, vh);
    Object.assign(pop.style, {
      position: "fixed",
      right: "auto",
      top: `${place.top}px`,
      left: `${place.left}px`,
    });
  }, [anchor, step]);

  useEffect(() => {
    const close = () => setStep("closed");
    const outside = (e: Event) => {
      const t = e.target;
      if (t instanceof Node && !popup.current?.contains(t) && !toggle?.current?.contains(t)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    // A touch, not just a click: a scroll or a swipe that closes the drawer sends no click.
    document.addEventListener("click", outside, true);
    document.addEventListener("contextmenu", outside, true);
    document.addEventListener("touchstart", outside, { capture: true, passive: true });
    document.addEventListener("keydown", onKey);
    window.addEventListener("scroll", close, true);
    return () => {
      document.removeEventListener("click", outside, true);
      document.removeEventListener("contextmenu", outside, true);
      document.removeEventListener("touchstart", outside, { capture: true });
      document.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", close, true);
    };
  }, [setStep, toggle]);

  return createPortal(
    // biome-ignore lint/a11y/useKeyWithClickEvents: only keeps a click in the portal from reaching the row.
    // biome-ignore lint/a11y/noStaticElementInteractions: see above.
    <div ref={popup} className="popover menu row-menu" onClick={(e) => e.stopPropagation()}>
      {step === "menu" ? (
        <>
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
