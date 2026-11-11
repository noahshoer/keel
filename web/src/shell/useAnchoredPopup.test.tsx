import { fireEvent, render } from "@testing-library/react";
import { useRef } from "react";
import { expect, it, vi } from "vitest";
import { useAnchoredPopup } from "./useAnchoredPopup";

// biome-ignore lint/style/useComponentExportOnlyModules: a test harness, never hot-reloaded.
function Probe({
  close,
  closeOnScroll,
  layoutKey,
  onPlace,
}: {
  close: () => void;
  closeOnScroll?: boolean;
  layoutKey?: unknown;
  onPlace?: () => void;
}) {
  const anchor = useRef<HTMLButtonElement>(null);
  const popup = useRef<HTMLDivElement>(null);
  useAnchoredPopup({
    anchor,
    popup,
    close,
    ...(closeOnScroll === undefined ? {} : { closeOnScroll }),
    layoutKey,
  });
  return (
    <>
      <button ref={anchor} type="button">
        a
      </button>
      <div
        ref={(el) => {
          popup.current = el;
          if (el)
            el.getBoundingClientRect = () => {
              onPlace?.();
              return new DOMRect();
            };
        }}
      >
        p
      </div>
    </>
  );
}

it("closes on a scroll by default", () => {
  const close = vi.fn();
  render(<Probe close={close} />);
  fireEvent.scroll(window);
  expect(close).toHaveBeenCalled();
});

it("stays open on a scroll when told not to", () => {
  const close = vi.fn();
  render(<Probe close={close} closeOnScroll={false} />);
  fireEvent.scroll(window);
  expect(close).not.toHaveBeenCalled();
  fireEvent.keyDown(document, { key: "Escape" });
  expect(close).toHaveBeenCalled();
});

it("is placed again when the layout key changes", () => {
  const onPlace = vi.fn();
  const close = vi.fn();
  const { rerender } = render(<Probe close={close} layoutKey={1} onPlace={onPlace} />);
  const before = onPlace.mock.calls.length;
  rerender(<Probe close={close} layoutKey={1} onPlace={onPlace} />);
  expect(onPlace.mock.calls.length).toBe(before);
  rerender(<Probe close={close} layoutKey={2} onPlace={onPlace} />);
  expect(onPlace.mock.calls.length).toBeGreaterThan(before);
});
