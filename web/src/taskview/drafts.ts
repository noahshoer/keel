// Rules for the task view's title and description drafts.
import type { EditorValues } from "../bindings";

/** Whether the title or description draft has changes to save before the view moves on. */
export function pendingSaveBeforeNav(before: EditorValues, draft: EditorValues): boolean {
  return before.content !== draft.content || before.description !== draft.description;
}

/**
 * Whether a draft takes the stored text after a reload: only when the stored
 * text changed (`seen` is the last stored text) and isn't just our own save
 * coming back (`saved`), so a draft typed since stays.
 */
export function takesStoredText(stored: string, seen: string, saved: string): boolean {
  return stored !== seen && stored !== saved;
}

/**
 * What a text field's new value means. A multiline field keeps its line
 * breaks (as `\n`) and never commits on them. Otherwise line breaks become
 * spaces and one counts as Enter (iOS sometimes inserts it rather than
 * sending a keydown the field can catch); the space it leaves at the end goes.
 */
export function typedText(raw: string, multiline: boolean): { text: string; enter: boolean } {
  if (multiline) {
    return { text: raw.replace(/\r\n/g, "\n").replace(/\r/g, "\n"), enter: false };
  }
  const enter = /[\n\r]/.test(raw);
  const text = raw.replace(/\r\n/g, " ").replace(/[\n\r]/g, " ");
  return { text: enter ? text.trimEnd() : text, enter };
}

/**
 * The title / description edit to save, if any. The title is trimmed; an
 * emptied one goes back to the saved one.
 */
export function commitDrafts(
  base: EditorValues,
  title: string,
  desc: string,
): { title: string; save: { before: EditorValues; after: EditorValues } | null } {
  const typed = title.trim();
  const shown = typed === "" ? base.content : typed;
  const after: EditorValues = { ...base, content: shown, description: desc };
  return { title: shown, save: pendingSaveBeforeNav(base, after) ? { before: base, after } : null };
}
