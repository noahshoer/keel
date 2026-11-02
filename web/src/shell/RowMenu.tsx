// A project, label or filter row's ••• button and its menu: Edit, then Delete,
// which asks first in the same popup. A long press on the row opens it too
// (gestures/long-press.ts, through `.nav-more`). Portaled, since the row scales
// while pressed and a fixed element inside it would move with it.
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { MoreIcon, PencilIcon, TrashSmallIcon } from "./icons";
import { placeRowMenu } from "./placeRowMenu";

type Step = "closed" | "menu" | "confirm";

export function RowMenu({
  warning,
  onEdit,
  onDelete,
}: {
  warning: string;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const [step, setStep] = useState<Step>("closed");
  const button = useRef<HTMLButtonElement>(null);
  const popup = useRef<HTMLDivElement>(null);
  const open = step !== "closed";

  // biome-ignore lint/correctness/useExhaustiveDependencies: the confirm is a different size from the menu.
  useLayoutEffect(() => {
    const row = button.current?.closest(".nav-item");
    const pop = popup.current;
    if (!row || !pop) return;
    const vh = window.visualViewport?.height ?? innerHeight;
    const at = placeRowMenu(row.getBoundingClientRect(), pop.getBoundingClientRect(), innerWidth, vh);
    Object.assign(pop.style, { position: "fixed", right: "auto", top: `${at.top}px`, left: `${at.left}px` });
  }, [step]);

  useEffect(() => {
    if (!open) return;
    const close = () => setStep("closed");
    const outside = (e: Event) => {
      const t = e.target;
      if (t instanceof Node && !popup.current?.contains(t) && !button.current?.contains(t)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    // A touch, not just a click: a scroll or a swipe that closes the drawer sends no click.
    document.addEventListener("click", outside, true);
    document.addEventListener("touchstart", outside, { capture: true, passive: true });
    document.addEventListener("keydown", onKey);
    window.addEventListener("scroll", close, true);
    return () => {
      document.removeEventListener("click", outside, true);
      document.removeEventListener("touchstart", outside, { capture: true });
      document.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", close, true);
    };
  }, [open]);

  return (
    <>
      <button
        ref={button}
        type="button"
        className="icon-btn nav-more"
        title="More"
        aria-label="More"
        onClick={(e) => {
          e.stopPropagation();
          setStep((s) => (s === "closed" ? "menu" : "closed"));
        }}
      >
        <MoreIcon />
      </button>
      {open
        ? createPortal(
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
              ) : (
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
              )}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
