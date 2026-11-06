// How the quick-add phrases in a title are painted and where backspace turns one plain.
import type { CSSProperties } from "react";
import type { Label, TitleSegment } from "../bindings";
import { BUCKET_CLASS } from "../lists/chips";
import { labelTagColor } from "./labelColors";

/**
 * A highlight's classes, from the same color its pill has: a date's due bucket,
 * a priority's level, a deadline red once it's urgent. A label's own color
 * comes from [`markStyle`].
 */
export function markClass(s: TitleSegment): string {
  switch (s.kind) {
    case "Date":
    case "Time":
      return `hl hl-due-${BUCKET_CLASS[s.bucket ?? "Later"]}`;
    case "Priority":
      return `hl hl-p${s.level}`;
    case "Deadline":
      return s.urgent ? "hl hl-deadline hl-urgent" : "hl hl-deadline";
    case "Project":
    case "Section":
      return "hl hl-project";
    case "Repeat":
      return "hl hl-repeat";
    case "Label":
      return "hl hl-label";
    case "Streak":
      return "hl hl-streak";
    default:
      return "hl";
  }
}

/** A label phrase's color (`@home`), `undefined` when the label has none or doesn't exist yet. */
export function markStyle(s: TitleSegment, labels: Label[]): CSSProperties | undefined {
  if (s.kind !== "Label") return undefined;
  const name = s.text.replace(/^@/, "").toLowerCase();
  const found = labels.find((l) => l.name.toLowerCase() === name);
  const color = found === undefined ? undefined : labelTagColor(found.color);
  return color === undefined ? undefined : ({ "--hl-color": color } as CSSProperties);
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
