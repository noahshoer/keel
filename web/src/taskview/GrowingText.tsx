// A text field that grows with its text: a textarea over a mirror
// (crates/ui/src/task_view/view.rs `GrowingText`). Leaving it commits.
// Single-line: Enter leaves the field (the keyboard goes), and line breaks are
// typed as spaces. Multiline: Enter is a new line.
import { useLayoutEffect, useRef } from "react";
import { log } from "../diag";
import { typedText } from "./drafts";

export function GrowingText({
  className,
  value,
  placeholder,
  multiline = false,
  autoFocus = false,
  onInput,
  onCommit,
}: {
  className: "tv-title" | "tv-desc";
  value: string;
  placeholder: string;
  multiline?: boolean;
  /** Focus on mount. A tap that opens the field renders it in `flushSync`, so this lands inside the tap and iOS raises the keyboard. */
  autoFocus?: boolean;
  onInput: (text: string) => void;
  onCommit: () => void;
}) {
  const field = useRef<HTMLTextAreaElement>(null);
  // Mount only, as Dioxus' `onmounted`.
  // biome-ignore lint/correctness/useExhaustiveDependencies: focuses once, when the field appears.
  useLayoutEffect(() => {
    if (autoFocus) field.current?.focus();
  }, []);
  // Enter: the blur that follows commits. A field that isn't focused has no
  // blur coming, so it commits now.
  const leave = () => {
    const el = field.current;
    if (el !== null && el === document.activeElement) el.blur();
    else onCommit();
  };
  return (
    <div className={`tv-grow ${className}`}>
      <div className="tv-grow-mirror" aria-hidden="true">
        {`${value}​`}
      </div>
      <textarea
        ref={field}
        rows={1}
        placeholder={placeholder}
        value={value}
        onChange={(e) => {
          const { text, enter } = typedText(e.target.value, multiline);
          onInput(text);
          if (enter) {
            log(`${className}: newline typed`);
            leave();
          }
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !multiline) {
            log(`${className}: Enter keydown`);
            e.preventDefault();
            leave();
          }
        }}
        onBlur={onCommit}
      />
    </div>
  );
}
