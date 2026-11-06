// A text field that grows with its text: a textarea over a mirror
// (crates/ui/src/task_view/view.rs `GrowingText`). Leaving it commits.
// Single-line: Enter leaves the field (the keyboard goes), and line breaks are
// typed as spaces. Multiline: Enter is a new line.
import { useLayoutEffect, useRef } from "react";
import type { Label, TitleSegment } from "../bindings";
import { log } from "../diag";
import { highlightEndsAt, markClass, markStyle } from "../editor/titleMarks";
import { typedText } from "./drafts";

export function GrowingText({
  className,
  value,
  placeholder,
  multiline = false,
  autoFocus = false,
  segments,
  labels = [],
  onUnhighlight,
  onInput,
  onCommit,
}: {
  className: "tv-title" | "tv-desc";
  value: string;
  placeholder: string;
  multiline?: boolean;
  /** Focus on mount. A tap that opens the field renders it in `flushSync`, so this lands inside the tap and iOS raises the keyboard. */
  autoFocus?: boolean;
  /** The phrases quick add recognized, painted behind the text. */
  segments?: TitleSegment[];
  /** For a label phrase's color. */
  labels?: Label[];
  /** Backspace right after a highlight turns it back into text. */
  onUnhighlight?: (caret: number) => void;
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
      {segments !== undefined ? (
        <div className="tv-grow-marks" aria-hidden="true">
          {segments.map((s, i) =>
            s.kind === null ? (
              // biome-ignore lint/suspicious/noArrayIndexKey: segments are positional pieces of one string.
              <span key={i}>{s.text}</span>
            ) : (
              // biome-ignore lint/suspicious/noArrayIndexKey: segments are positional pieces of one string.
              <mark key={i} className={markClass(s)} style={markStyle(s, labels)}>
                {s.text}
              </mark>
            ),
          )}
          {"​"}
        </div>
      ) : null}
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
            return;
          }
          const ta = e.currentTarget;
          if (
            e.key === "Backspace" &&
            segments !== undefined &&
            onUnhighlight &&
            ta.selectionStart === ta.selectionEnd
          ) {
            if (highlightEndsAt(segments, ta.selectionStart)) {
              e.preventDefault();
              onUnhighlight(ta.selectionStart);
            }
          }
        }}
        onBlur={onCommit}
      />
    </div>
  );
}
