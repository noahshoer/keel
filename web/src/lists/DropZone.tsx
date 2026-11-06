// Where a dragged task can land: a project's own tasks, or one of its sections.
// `drag.ts` finds these by their `data-drop-*` attributes (the project, the
// section or "" for none, and the name the page says it'll "Drop to move to"),
// and sends `TASK_DROP` to the one a task is let go over; it moves the task here.
import { type ReactNode, useEffect, useRef } from "react";
import type { Id } from "../bindings";
import { TASK_DROP } from "../gestures/drag";
import { moveTask } from "../ipc/actions";

export function DropZone({
  project,
  section,
  name,
  className,
  onChanged,
  children,
}: {
  project: Id;
  section: Id | null;
  name: string;
  className?: string;
  onChanged: () => void;
  children: ReactNode;
}) {
  const zone = useRef<HTMLDivElement>(null);
  const latest = useRef(onChanged);
  latest.current = onChanged;
  useEffect(() => {
    const el = zone.current;
    if (!el) return;
    const dropped = (e: Event) => {
      const { taskId } = (e as CustomEvent<{ taskId: string }>).detail;
      moveTask(taskId, project, section).then(latest.current, latest.current);
    };
    el.addEventListener(TASK_DROP, dropped);
    return () => el.removeEventListener(TASK_DROP, dropped);
  }, [project, section]);
  return (
    <div
      ref={zone}
      className={className}
      data-drop-zone=""
      data-drop-project={project}
      data-drop-section={section ?? ""}
      data-drop-name={name}
    >
      {children}
    </div>
  );
}
