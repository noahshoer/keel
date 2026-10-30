// The task view's sheet (crates/ui/src/main.rs `App`, the `open_task`
// branch): a scrim and a sheet holding the open view, and the one under it
// when a sub-task is on top. app.css makes it a sheet on the phone and a
// right panel on wide windows. The scrim and sheet stay mounted while views
// push and pop inside, so the entrance plays once per open.
import { useMemo } from "react";
import type { Loaded } from "../app/load";
import type { Id, Task, TaskDetail } from "../bindings";
import { taskDetail } from "../core";
import { slideClose } from "../gestures/sheet";
import { useTaskNav } from "./nav-context";
import { top, under } from "./stack";
import { TaskView } from "./TaskView";

/**
 * Each task's detail, computed once per (id, `all`, `showCompleted`): the
 * view's stored values are keyed on the task object, and a task that moves
 * from top to under on a push keeps the same one.
 */
function useDetails(all: Task[], showCompleted: boolean): (id: Id | null) => TaskDetail | null {
  const cache = useMemo(
    () => ({ all, showCompleted, byId: new Map<Id, TaskDetail | null>() }),
    [all, showCompleted],
  );
  return (id) => {
    if (id === null) return null;
    let d = cache.byId.get(id);
    if (d === undefined) {
      d = taskDetail(id, cache.all, cache.showCompleted);
      cache.byId.set(id, d);
    }
    return d;
  };
}

export function TaskSheet({ loaded, onChanged }: { loaded: Loaded; onChanged: () => void }) {
  const nav = useTaskNav();
  const detail = useDetails(loaded.all, nav.showCompleted);
  const open = detail(top(nav.stack));
  if (open === null) return null;
  // The view under it too, when a sub-task is on top: it stays mounted
  // (hidden) so the edge swipe slides it in, and a Back keeps it as is.
  const below = detail(under(nav.stack));
  const levels: Array<{ d: TaskDetail; active: boolean }> = [{ d: open, active: true }];
  if (below !== null) levels.unshift({ d: below, active: false });
  return (
    <>
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: a pointer target, as in the Dioxus UI. */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI. */}
      <div
        className="tv-scrim"
        onClick={() => {
          if (!slideClose(document, window)) nav.askClose();
        }}
      />
      <div className="tv-sheet">
        <div className="tv-grabber" aria-hidden="true" />
        <div className="tv-stack">
          {/* Keyed by task, in stack order: a push or a Back keeps the views that stay mounted. */}
          {levels.map(({ d, active }) => (
            <TaskView
              key={d.task.id}
              detail={d}
              depth={nav.stack.length - (active ? 0 : 1)}
              active={active}
              loaded={loaded}
              onChanged={onChanged}
            />
          ))}
        </div>
      </div>
    </>
  );
}
