// The shell of the filter and sort panels: a bottom sheet under 900px, a
// popover hanging from its header button from 900px up.
import { type ReactNode, type RefObject, useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { useWideLayout } from "../shell/platform";
import { useAnchoredPopup } from "../shell/useAnchoredPopup";

export type ArrangePanelProps = {
  title: string;
  action: { label: string; run: () => void };
  /** The header button it opens from. */
  anchor: RefObject<HTMLElement | null>;
  /** Must keep its identity. */
  onClose: () => void;
  /** The popover is placed again when this changes (its content's size does). */
  layoutKey?: unknown;
  children: ReactNode;
};

function Body({ title, action, children }: Omit<ArrangePanelProps, "anchor" | "onClose" | "layoutKey">) {
  return (
    <>
      <div className="arrange-panel-head">
        <h2 className="arrange-panel-title">{title}</h2>
        <button type="button" className="arrange-panel-action" onClick={action.run}>
          {action.label}
        </button>
      </div>
      <div className="arrange-panel-body">{children}</div>
    </>
  );
}

function Popover(props: ArrangePanelProps) {
  const popup = useRef<HTMLDivElement>(null);
  // Not on a scroll: the panel scrolls itself, and the list behind it re-flows as a filter is typed.
  useAnchoredPopup({
    anchor: props.anchor,
    popup,
    toggle: props.anchor,
    close: props.onClose,
    layoutKey: props.layoutKey,
    closeOnScroll: false,
  });
  return (
    <div ref={popup} className="arrange-panel arrange-popover" role="dialog" aria-label={props.title}>
      <Body {...props} />
    </div>
  );
}

function Sheet(props: ArrangePanelProps) {
  const { onClose } = props;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: Escape closes it (above); the scrim is a pointer target only.
    // biome-ignore lint/a11y/useKeyWithClickEvents: same.
    <div
      className="arrange-scrim"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="arrange-panel arrange-sheet" role="dialog" aria-label={props.title}>
        <div className="tv-grabber" />
        <Body {...props} />
      </div>
    </div>
  );
}

export function ArrangePanel(props: ArrangePanelProps) {
  // On the body, outside the header's own layers, so the add button can't draw over it.
  return createPortal(useWideLayout() ? <Popover {...props} /> : <Sheet {...props} />, document.body);
}
