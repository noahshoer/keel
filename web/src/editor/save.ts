// Saving the editor's values: keel-core plans what to send (`addPlan`,
// `editPlan`), and this sends it in order, as crates/ui/src/editor.rs
// `add_from_editor` and `save_edit` do.
import type { EditorValues, Id, Label, Project, Recurrence, Section } from "../bindings";
import { addPlan, editPlan, reminderActions } from "../core";
import { addLabel, dispatchAction } from "../ipc/actions";

/** Sends each action, ignoring failures (reminders and labels are best effort). */
async function bestEffort(sends: Array<() => Promise<unknown>>): Promise<void> {
  for (const send of sends) {
    try {
      await send();
    } catch {
      // The task itself is saved; a missing reminder or label isn't worth failing for.
    }
  }
}

/**
 * Adds the task the editor describes (quick add applied), creating any labels
 * it mentions that don't exist yet. Rejects when the task wasn't added.
 */
export async function addFromEditor(
  v: EditorValues,
  projects: Project[],
  sections: Section[],
  labels: Label[],
  today: string,
): Promise<void> {
  const plan = addPlan(v, projects, sections, labels, today);
  await bestEffort(plan.labels.map((name) => () => addLabel(name)));
  const result = await dispatchAction(plan.task, true);
  const id = Object.values(result.id_mapping)[0];
  if (id === undefined) return;
  await bestEffort(reminderActions(id, [], plan.reminders).map((a) => () => dispatchAction(a)));
}

/**
 * Saves an edited task: the move (with its sub-tree), reminder changes, then
 * the field changes. Rejects when the move or the field changes weren't saved.
 */
export async function saveEdit(
  id: Id,
  before: EditorValues,
  after: EditorValues,
  existing: Recurrence | null,
  today: string,
): Promise<void> {
  const plan = editPlan(id, before, after, existing, today);
  if (plan.moved !== null) await dispatchAction(plan.moved);
  await bestEffort(plan.reminders.map((a) => () => dispatchAction(a)));
  if (plan.update !== null) await dispatchAction(plan.update);
}
