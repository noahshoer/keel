// The task title: an auto-growing textarea over a mirror that paints the
// quick-add phrases keel-core recognized (crates/ui/src/editor.rs
// `HighlightedTitle`). Enter submits (titles are one line). Backspace right
// where a highlight ends turns it back into text, as in Todoist.
import type { RefObject } from "react";
import type { SpanKind, TitleSegment } from "../bindings";

const MARK: Record<SpanKind, string> = {
  Date: "hl hl-date",
  Time: "hl hl-date",
  Repeat: "hl hl-repeat",
  Priority: "hl hl-p4",
  Project: "hl hl-project",
  Section: "hl hl-project",
  Label: "hl hl-label",
  Deadline: "hl hl-deadline",
};

/** A highlight's class; a priority's colour is its level ("p2" → `hl-p2`). */
function markClass(kind: SpanKind, level: number): string {
  return kind === "Priority" ? `hl hl-p${level}` : MARK[kind];
}

/** Whether a highlight ends exactly at `caret` (UTF-16, as the textarea counts). */
function highlightEndsAt(segments: TitleSegment[], caret: number): boolean {
  let at = 0;
  for (const s of segments) {
    at += s.text.length;
    if (at === caret && s.kind !== null) return true;
    if (at >= caret) return false;
  }
  return false;
}

export function HighlightedTitle({
  segments,
  value,
  placeholder,
  onInput,
  onEnter,
  onUnhighlight,
  inputRef,
}: {
  segments: TitleSegment[];
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
            <mark key={i} className={markClass(s.kind, s.level)}>
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
