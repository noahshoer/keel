// What editing and deleting a project, label or filter does, from its sidebar
// row or its page: the change itself, and keeping the open view in step with it.
import type { View } from "../app/view";
import type { Filter, Label, Project } from "../bindings";
import {
  deleteFilter,
  deleteLabel,
  deleteProject,
  renameLabel,
  renameProject,
  updateFilter,
} from "../ipc/actions";

export type ListContext = {
  view: View;
  setView: (v: View) => void;
  /** Reloads once the change lands, whether or not it worked. */
  settle: (p: Promise<unknown>) => Promise<unknown>;
};

export type NameActions = { warning: string; rename: (name: string) => void; remove: () => void };
export type FilterActions = {
  warning: string;
  save: (name: string, query: string) => void;
  remove: () => void;
};

export function projectActions(p: Project, { view, setView, settle }: ListContext): NameActions {
  const open = view.kind === "project" && view.id === p.id;
  return {
    warning: `Delete “${p.name}” and all its tasks?`,
    rename: (name) => {
      void settle(renameProject(p.id, name));
      // Keep the open view's title in sync with the new name.
      if (open) setView({ kind: "project", id: p.id, name });
    },
    remove: () => {
      void settle(deleteProject(p.id));
      if (open) setView({ kind: "inbox" });
    },
  };
}

export function labelActions(l: Label, { view, setView, settle }: ListContext): NameActions {
  const open = view.kind === "label" && view.name.toLowerCase() === l.name.toLowerCase();
  return {
    warning: `Delete label “${l.name}”? It’s removed from its tasks.`,
    rename: (name) => {
      void settle(renameLabel(l.id, name));
      if (open) setView({ kind: "label", name });
    },
    remove: () => {
      void settle(deleteLabel(l.id));
      if (open) setView({ kind: "inbox" });
    },
  };
}

export function filterActions(f: Filter, { view, setView, settle }: ListContext): FilterActions {
  const open = view.kind === "filter" && view.id === f.id;
  return {
    warning: `Delete filter “${f.name}”? Your tasks aren’t affected.`,
    save: (name, query) => {
      void settle(updateFilter(f.id, name, query));
      // Keep the open view in sync with the edited filter.
      if (open) setView({ kind: "filter", id: f.id, name, query });
    },
    remove: () => {
      void settle(deleteFilter(f.id));
      if (open) setView({ kind: "inbox" });
    },
  };
}
