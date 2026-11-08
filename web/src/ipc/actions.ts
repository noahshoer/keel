// Every change goes through `dispatch` as a Command (crates/core/src/commands.rs):
// a fresh uuid makes it idempotent, and creates carry a temp id the backend
// maps to the new id (`ApplyResult.id_mapping`). The same helpers and temp
// flags as crates/ui/src/api.rs.
import type { Action, ApplyResult, Command, Id } from "../bindings";
import { invoke } from "./invoke";

export function command(action: Action, temp: boolean): Command {
  return { uuid: crypto.randomUUID(), temp_id: temp ? `tmp-${crypto.randomUUID()}` : null, action };
}

/** Sends one action; `temp` gives a create its temp id. */
export function dispatchAction(action: Action, temp = false): Promise<ApplyResult> {
  return invoke("dispatch", { cmd: command(action, temp) });
}

const send = dispatchAction;

export const completeTask = (id: Id) => send({ type: "task_complete", id });
export const uncompleteTask = (id: Id) => send({ type: "task_uncomplete", id });
export const deleteTask = (id: Id) => send({ type: "task_delete", id });
/** Moves a task (and its sub-tasks) to a project, into a section or out of any. */
export const moveTask = (id: Id, projectId: Id, sectionId: Id | null) =>
  send({ type: "task_move", id, project_id: projectId, section_id: sectionId });
export const setStreak = (id: Id, enabled: boolean, goal: number | null) =>
  send({ type: "task_set_streak", id, enabled, goal });
/** Turns on/off a task completing itself once its sub-tasks are all done. */
export const setDoneWithSubtasks = (id: Id, enabled: boolean) =>
  send({ type: "task_set_done_with_subtasks", id, enabled });

export const addProject = (name: string) => send({ type: "project_add", name }, true);
export const renameProject = (id: Id, name: string) => send({ type: "project_update", id, name });
export const deleteProject = (id: Id) => send({ type: "project_delete", id });

export const addLabel = (name: string) => send({ type: "label_add", name }, true);
export const renameLabel = (id: Id, name: string) => send({ type: "label_update", id, name });
export const setLabelColor = (id: Id, color: string) => send({ type: "label_set_color", id, color });
export const deleteLabel = (id: Id) => send({ type: "label_delete", id });

export const addSection = (projectId: Id, name: string) =>
  send({ type: "section_add", project_id: projectId, name }, true);
/** Adds a section and gives its id (to move a task into it). */
export async function addSectionGetId(projectId: Id, name: string): Promise<Id> {
  const cmd = command({ type: "section_add", project_id: projectId, name }, true);
  const result = await invoke("dispatch", { cmd });
  const id = cmd.temp_id ? result.id_mapping[cmd.temp_id] : undefined;
  if (id === undefined) throw new Error("the new section has no id");
  return id;
}
export const renameSection = (id: Id, name: string) => send({ type: "section_update", id, name });
export const deleteSection = (id: Id) => send({ type: "section_delete", id });

export const addFilter = (name: string, query: string) => send({ type: "filter_add", name, query }, true);
export const updateFilter = (id: Id, name: string, query: string) =>
  send({ type: "filter_update", id, name, query });
export const deleteFilter = (id: Id) => send({ type: "filter_delete", id });
