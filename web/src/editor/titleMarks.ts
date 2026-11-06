// How the quick-add phrases in a title are painted and where backspace turns one plain.
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
  Streak: "hl hl-date",
};

/** A highlight's class; a priority's colour is its level ("p2" → `hl-p2`). */
export function markClass(kind: SpanKind, level: number): string {
  return kind === "Priority" ? `hl hl-p${level}` : MARK[kind];
}

/** Whether a highlight ends exactly at `caret` (UTF-16, as the textarea counts). */
export function highlightEndsAt(segments: TitleSegment[], caret: number): boolean {
  let at = 0;
  for (const s of segments) {
    at += s.text.length;
    if (at === caret && s.kind !== null) return true;
    if (at >= caret) return false;
  }
  return false;
}
