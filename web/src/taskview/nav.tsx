// The task view's navigation state (crates/ui/src/task_view/nav.rs
// `TaskViewNav`): the stack of open tasks and the view's own flags.
import { type ReactNode, useCallback, useMemo, useRef, useState } from "react";
import type { Id, Task } from "../bindings";
import { CIRCLE_SETTLE_MS, type TaskNav, TaskNavContext } from "./nav-context";
import * as stacks from "./stack";

/** Provides the nav; `all` is every task, refreshed with each load (`open` walks it for ancestors). */
export function TaskNavProvider({ all, children }: { all: Task[]; children: ReactNode }) {
  const [stack, setStack] = useState<readonly Id[]>([]);
  const [showCompleted, setShowCompleted] = useState(false);
  const [addSub, setAddSub] = useState(false);
  const [closeAsked, setCloseAsked] = useState(false);
  // Refs, so the functions stay the same and a second synchronous call sees the first.
  const allRef = useRef(all);
  allRef.current = all;
  const stackRef = useRef(stack);
  stackRef.current = stack;
  const held = useRef(false);

  const open = useCallback((id: Id) => {
    setShowCompleted(false);
    setStack(stacks.openChain(id, allRef.current));
  }, []);
  const openAdding = useCallback(
    (id: Id) => {
      open(id);
      setAddSub(true);
    },
    [open],
  );
  const push = useCallback((id: Id) => setStack((s) => stacks.push(s, id)), []);
  const back = useCallback(() => setStack((s) => stacks.back(s)), []);
  const close = useCallback(() => setStack([]), []);
  const askClose = useCallback(() => {
    if (stackRef.current.length > 0) setCloseAsked(true);
  }, []);
  const clearAddSub = useCallback(() => setAddSub(false), []);
  const clearCloseAsked = useCallback(() => setCloseAsked(false), []);
  const holdCircles = useCallback(() => {
    if (held.current) return false;
    held.current = true;
    return true;
  }, []);
  const releaseCircles = useCallback(
    () =>
      new Promise<void>((resolve) =>
        setTimeout(() => {
          held.current = false;
          resolve();
        }, CIRCLE_SETTLE_MS),
      ),
    [],
  );
  const prune = useCallback((tasks: Task[]) => setStack((s) => stacks.prune(s, tasks)), []);

  const value = useMemo<TaskNav>(
    () => ({
      stack,
      showCompleted,
      addSub,
      closeAsked,
      open,
      openAdding,
      push,
      back,
      close,
      askClose,
      setShowCompleted,
      clearAddSub,
      clearCloseAsked,
      completedHere: back,
      holdCircles,
      releaseCircles,
      prune,
    }),
    [
      stack,
      showCompleted,
      addSub,
      closeAsked,
      open,
      openAdding,
      push,
      back,
      close,
      askClose,
      clearAddSub,
      clearCloseAsked,
      holdCircles,
      releaseCircles,
      prune,
    ],
  );
  return <TaskNavContext.Provider value={value}>{children}</TaskNavContext.Provider>;
}
