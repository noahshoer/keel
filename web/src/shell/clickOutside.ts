// A form in the sidebar closes on a click outside it. `click`, not a press, so
// a scroll that starts outside leaves it open and the click still reaches the
// row it landed on. Capture, so the click that opened the form (already past
// the document on its way in) doesn't close it.
import { type RefObject, useEffect, useRef } from "react";

/** The attribute on a button that opens and closes a form by itself. */
export const FORM_TOGGLE = "data-form-toggle";

export function useCloseOnClickOutside(form: RefObject<HTMLElement | null>, close: () => void): void {
  const latest = useRef(close);
  useEffect(() => {
    latest.current = close;
  });
  useEffect(() => {
    const inForm = (e: Event) => e.target instanceof Node && form.current?.contains(e.target) === true;
    // A drag from the field (selecting its text) ends in a click on whatever it ends over.
    let startedInForm = false;
    const onPointerDown = (e: PointerEvent) => {
      startedInForm = inForm(e);
    };
    const onClick = (e: MouseEvent) => {
      const onToggle = e.target instanceof Element && e.target.closest(`[${FORM_TOGGLE}]`) !== null;
      if (!startedInForm && !inForm(e) && !onToggle) latest.current();
      startedInForm = false;
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("click", onClick, true);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("click", onClick, true);
    };
  }, [form]);
}
