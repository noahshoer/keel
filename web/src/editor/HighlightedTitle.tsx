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
}) {
  return (
    <div className="title-wrap">
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
        // biome-ignore lint/a11y/noAutofocus: the editor opens to type a title, as the Dioxus one does.
        autoFocus
        spellCheck={false}
        value={value}
        onChange={(e) => onInput(e.target.value.replace(/[\n\r]/g, " "))}
        onKeyDown={(e) => {
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
