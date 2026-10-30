// Every change goes through `dispatch` as a Command (crates/core/src/commands.rs):
// a fresh uuid makes it idempotent, and creates carry a temp id the backend
// maps to the new id (`ApplyResult.id_mapping`). The same helpers and temp
// flags as crates/ui/src/api.rs.
import type { Action, ApplyResult, Command, Id } from "../bindings";
import { invoke } from "./invoke";

export function command(action: Action, temp: boolean): Command {
  return { uuid: crypto.randomUUID(), temp_id: temp ? `tmp-${crypto.randomUUID()}` : null, action };
}

function send(action: Action, temp = false): Promise<ApplyResult> {
  return invoke("dispatch", { cmd: command(action, temp) });
}

export const completeTask = (id: Id) => send({ type: "task_complete", id });
export const uncompleteTask = (id: Id) => send({ type: "task_uncomplete", id });
export const deleteTask = (id: Id) => send({ type: "task_delete", id });
export const setStreak = (id: Id, enabled: boolean, goal: number | null) =>
  send({ type: "task_set_streak", id, enabled, goal });

export const addProject = (name: string) => send({ type: "project_add", name }, true);
export const renameProject = (id: Id, name: string) => send({ type: "project_update", id, name });
export const deleteProject = (id: Id) => send({ type: "project_delete", id });

export const addLabel = (name: string) => send({ type: "label_add", name }, true);
export const renameLabel = (id: Id, name: string) => send({ type: "label_update", id, name });
export const deleteLabel = (id: Id) => send({ type: "label_delete", id });

export const addSection = (projectId: Id, name: string) =>
  send({ type: "section_add", project_id: projectId, name }, true);
export const renameSection = (id: Id, name: string) => send({ type: "section_update", id, name });
export const deleteSection = (id: Id) => send({ type: "section_delete", id });

export const addFilter = (name: string, query: string) => send({ type: "filter_add", name, query }, true);
export const updateFilter = (id: Id, name: string, query: string) =>
  send({ type: "filter_update", id, name, query });
export const deleteFilter = (id: Id) => send({ type: "filter_delete", id });
