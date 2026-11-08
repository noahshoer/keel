// The task title: an auto-growing textarea over a mirror that paints the
// quick-add phrases keel-core recognized (crates/ui/src/editor.rs
// `HighlightedTitle`). Enter submits (titles are one line). Backspace right
// where a highlight ends turns it back into text, as in Todoist.
import type { RefObject } from "react";
import type { Label, TitleSegment } from "../bindings";
import { highlightEndsAt, markClass, markStyle } from "./titleMarks";

export function HighlightedTitle({
  segments,
  labels = [],
  value,
  placeholder,
  onInput,
  onEnter,
  onUnhighlight,
  inputRef,
  autoFocus = true,
  label,
  onEscape,
  className,
  invalid = false,
  describedBy,
}: {
  segments: TitleSegment[];
  /** For a label phrase's color. */
  labels?: Label[];
  value: string;
  placeholder: string;
  onInput: (text: string) => void;
  onEnter: () => void;
  /** Turns the highlight ending at `caret` plain. */
  onUnhighlight: (caret: number) => void;
  inputRef?: RefObject<HTMLTextAreaElement | null> | undefined;
  /** Focus on mount (the editor); a form field that sits among others turns it off. */
  autoFocus?: boolean;
  /** The textarea's accessible name. */
  label?: string | undefined;
  /** Escape pressed in the field. */
  onEscape?: () => void;
  /** Extra classes on the wrapper. */
  className?: string;
  /** The value has a problem (`aria-invalid`). */
  invalid?: boolean;
  /** The id of the line that describes the field (`aria-describedby`). */
  describedBy?: string | undefined;
}) {
  return (
    <div className={className ? `title-wrap ${className}` : "title-wrap"}>
      <div className="title-mirror" aria-hidden="true">
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
        {/* Keeps the last line's height when the value ends in a space. */}
        {"​"}
      </div>
      <textarea
        ref={inputRef}
        className="name title-input"
        rows={1}
        placeholder={placeholder}
        aria-label={label}
        aria-invalid={invalid || undefined}
        aria-describedby={describedBy}
        // biome-ignore lint/a11y/noAutofocus: the editor opens to type a title, as the Dioxus one does.
        autoFocus={autoFocus}
        spellCheck={false}
        value={value}
        onChange={(e) => onInput(e.target.value.replace(/[\n\r]/g, " "))}
        onKeyDown={(e) => {
          if (e.key === "Escape" && onEscape) {
            onEscape();
            return;
          }
          if (e.key === "Enter") {
            e.preventDefault();
            onEnter();
            return;
          }
          const ta = e.currentTarget;
          if (e.key !== "Backspace" || ta.selectionStart !== ta.selectionEnd) return;
          if (highlightEndsAt(segments, ta.selectionStart)) {
            e.preventDefault();
            onUnhighlight(ta.selectionStart);
          }
        }}
      />
    </div>
  );
}
