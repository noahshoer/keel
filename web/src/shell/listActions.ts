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
  /** The open view under its new name: the same page, so it doesn't fade in again. */
  renameView: (v: View) => void;
  /** Reloads once the change lands, whether or not it worked. */
  settle: (p: Promise<unknown>) => Promise<unknown>;
};

export type NameActions = { warning: string; rename: (name: string) => void; remove: () => void };
export type FilterActions = {
  warning: string;
  save: (name: string, query: string) => void;
  remove: () => void;
};

export function projectActions(p: Project, { view, setView, renameView, settle }: ListContext): NameActions {
  const open = view.kind === "project" && view.id === p.id;
  return {
    warning: `Delete “${p.name}” and all its tasks?`,
    rename: (name) => {
      void settle(renameProject(p.id, name));
      // Keep the open view's title in sync with the new name.
      if (open) renameView({ kind: "project", id: p.id, name });
    },
    remove: () => {
      void settle(deleteProject(p.id));
      if (open) setView({ kind: "inbox" });
    },
  };
}

export function labelActions(l: Label, { view, setView, renameView, settle }: ListContext): NameActions {
  const open = view.kind === "label" && view.name.toLowerCase() === l.name.toLowerCase();
  return {
    warning: `Delete label “${l.name}”? It’s removed from its tasks.`,
    rename: (name) => {
      void settle(renameLabel(l.id, name));
      if (open) renameView({ kind: "label", name });
    },
    remove: () => {
      void settle(deleteLabel(l.id));
      if (open) setView({ kind: "inbox" });
    },
  };
}

export function filterActions(f: Filter, { view, setView, renameView, settle }: ListContext): FilterActions {
  const open = view.kind === "filter" && view.id === f.id;
  return {
    warning: `Delete filter “${f.name}”? Your tasks aren’t affected.`,
    save: (name, query) => {
      void settle(updateFilter(f.id, name, query));
      // Keep the open view in sync with the edited filter.
      if (open) renameView({ kind: "filter", id: f.id, name, query });
    },
    remove: () => {
      void settle(deleteFilter(f.id));
      if (open) setView({ kind: "inbox" });
    },
  };
}

export type PageEditing =
  | { kind: "name"; name: string; rename: (name: string) => void }
  | { kind: "filter"; filter: Filter; save: (name: string, query: string) => void };

export type PageActions = { warning: string; remove: () => void; editing: PageEditing };

/** What a view's page offers: only a project, label or filter has actions. */
export function pageActions(
  lists: { projects: Project[]; labels: Label[]; filters: Filter[] },
  context: ListContext,
): PageActions | null {
  const { view } = context;
  if (view.kind === "project") {
    const project = lists.projects.find((p) => p.id === view.id);
    if (!project) return null;
    const { warning, rename, remove } = projectActions(project, context);
    return { warning, remove, editing: { kind: "name", name: project.name, rename } };
  }
  if (view.kind === "label") {
    const label = lists.labels.find((l) => l.name.toLowerCase() === view.name.toLowerCase());
    if (!label) return null;
    const { warning, rename, remove } = labelActions(label, context);
    return { warning, remove, editing: { kind: "name", name: label.name, rename } };
  }
  if (view.kind === "filter") {
    const filter = lists.filters.find((f) => f.id === view.id);
    if (!filter) return null;
    const { warning, save, remove } = filterActions(filter, context);
    return { warning, remove, editing: { kind: "filter", filter, save } };
  }
  return null;
}
