// iOS shows the keyboard for a focus outside a tap (a task let go on Add section,
// focused at the end of the drag) only once the person has focused a field by a
// tap since the page loaded. So the first real tap anywhere focuses a hidden
// field that asks for no keyboard (`inputmode="none"`) and lets it go at once:
// from then on, a drop's focus raises the keyboard. Once per page; a tap on a
// field needs nothing, since that focus is already the person's.

const FIELDS = "input, textarea, select, [contenteditable]";

type Options = {
  /** Whether a click is a person's (a script's doesn't count). */
  trusted?: (e: MouseEvent) => boolean;
};

/** Installs the one-time listener on `doc`; returns a function that removes it. */
export function installKeyboardPrime(doc: Document, options: Options = {}): () => void {
  const trusted = options.trusted ?? ((e: MouseEvent) => e.isTrusted);
  const onClick = (e: MouseEvent) => {
    if (!trusted(e)) return;
    doc.removeEventListener("click", onClick, true);
    if (e.target instanceof Element && e.target.closest(FIELDS)) return;
    const field = doc.createElement("input");
    field.setAttribute("inputmode", "none");
    field.setAttribute("aria-hidden", "true");
    field.tabIndex = -1;
    Object.assign(field.style, {
      position: "fixed",
      top: "0",
      left: "0",
      width: "1px",
      height: "1px",
      opacity: "0",
      pointerEvents: "none",
    });
    doc.body.appendChild(field);
    field.focus({ preventScroll: true });
    field.blur();
    field.remove();
  };
  doc.addEventListener("click", onClick, true);
  return () => doc.removeEventListener("click", onClick, true);
}
