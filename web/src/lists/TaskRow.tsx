// One task in a list: the markup and behavior of crates/ui/src/task_row.rs
// `TaskRow` without its glass parts (Liquid Glass is parked). Tapping the
// circle completes the task after the fill and switch-off; the body opens
// the task view, and Add sub-task opens it adding.
import { useContext, useEffect, useRef, useState } from "react";
import type { CardRow, Progress, Reminder } from "../bindings";
import { allDone, deadlineUrgent, dueLabel, progressShort, reminderLabel } from "../core";
import { completeTask, deleteTask } from "../ipc/actions";
import {
  BellSmallIcon,
  CalendarSmallIcon,
  CheckSmallIcon,
  DoneMark,
  RepeatSmallIcon,
  SubtaskIcon,
  TagSmallIcon,
  TargetSmallIcon,
  TrashIcon,
} from "../shell/icons";
import { useTaskNav } from "../taskview/nav-context";
import { CHECK_CLASS, dueChip } from "./chips";
import { CompletionIconContext, FILL_MS, nextOnTap, OFF_MS, type Phase, sleep } from "./completion";
import { StreakPanel } from "./StreakPanel";

/** "x/y" for a task's sub-tasks, then a round green check when all are done. */
export function SubtaskCount({ progress }: { progress: Progress }) {
  return (
    <>
      <span className="task-count">{progressShort(progress)}</span>
      {allDone(progress) ? (
        <span className="done-check" title="All sub-tasks done">
          <CheckSmallIcon />
        </span>
      ) : null}
    </>
  );
}

const PHASE_CLASS: Record<Phase, string> = {
  0: "task",
  1: "task completing",
  2: "task completing switch-off",
};

export function TaskRow({
  row,
  today,
  location,
  reminders,
  onChanged,
}: {
  row: CardRow;
  today: string;
  /** "Project / Section", shown in views that mix projects. */
  location: string | null;
  /** Every task reminder; the row shows its own. */
  reminders: Reminder[];
  onChanged: () => void;
}) {
  const { task, progress } = row;
  const icon = useContext(CompletionIconContext);
  const nav = useTaskNav();
  const [phase, setPhase] = useState<Phase>(0);
  const [panel, setPanel] = useState(false);
  // A tap while filled cancels: each start bumps `gen`, and pending steps
  // stop when it has moved on.
  const gen = useRef(0);
  // Leaving the screen cancels a completion still filling or switching off.
  useEffect(
    () => () => {
      gen.current += 1;
    },
    [],
  );
  // After completing, the row stays switched off until the reload drops it.
  // A repeating task stays in project views with a new due date; that new
  // date brings it back.
  const [doneDue, setDoneDue] = useState<string | null>(null);
  const dueNow = task.due?.date ?? "";
  useEffect(() => {
    if (doneDue !== null && doneDue !== dueNow) {
      setDoneDue(null);
      setPhase(0);
    }
  }, [doneDue, dueNow]);

  const complete = async () => {
    const tap = nextOnTap(phase);
    if (tap === "ignore") return;
    gen.current += 1;
    if (tap === "cancel") {
      setPhase(0);
      return;
    }
    const mine = gen.current;
    setPhase(1);
    await sleep(FILL_MS);
    if (gen.current !== mine) return;
    setPhase(2);
    await sleep(OFF_MS);
    if (gen.current !== mine) return;
    // Hidden before the save so the reload can't flash it back; if the save
    // fails, the row returns unchecked.
    setDoneDue(dueNow);
    try {
      await completeTask(task.id);
    } catch {
      setDoneDue(null);
      setPhase(0);
    }
    onChanged();
  };

  const mine = reminders.filter((r) => r.task_id === task.id);
  const reminderTitle = mine.map((r) => reminderLabel(r, today)).join(", ");
  const deadline = task.deadline
    ? { cls: deadlineUrgent(task.deadline, today) ? "urgent" : "set", label: dueLabel(task.deadline, today) }
    : null;
  const due = dueChip(task, today);
  const recurring = task.due?.is_recurring ?? false;
  const showMeta = due !== null || task.labels.length > 0 || deadline !== null || mine.length > 0;

  return (
    // One root per row (keyed rows reorder cleanly when a priority changes).
    <div className={PHASE_CLASS[phase]} data-menu-id={task.id}>
      <div className="task-row">
        <button
          type="button"
          className={CHECK_CLASS[task.priority]}
          title="Complete task"
          onClick={() => void complete()}
        >
          <DoneMark icon={icon} />
        </button>
        {/* biome-ignore lint/a11y/useKeyWithClickEvents: same markup as the Dioxus UI. */}
        {/* biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI. */}
        <div className="task-body" title="Open task" onClick={() => nav.open(task.id)}>
          <div className="task-title">
            <div className="task-content">{task.content}</div>
          </div>
          {task.description ? <div className="task-desc">{task.description}</div> : null}
          {showMeta ? (
            <div className="task-meta">
              {due !== null ? (
                <span className={due.cls}>
                  <CalendarSmallIcon />
                  {due.label}
                  {recurring ? <RepeatSmallIcon /> : null}
                </span>
              ) : null}
              {deadline !== null ? (
                <span className={`deadline ${deadline.cls}`} title="Deadline">
                  <TargetSmallIcon />
                  {deadline.label}
                </span>
              ) : null}
              {mine.length > 0 ? (
                <span className="reminder-tag" title={reminderTitle}>
                  <BellSmallIcon />
                </span>
              ) : null}
              {task.labels.map((name) => (
                <span key={name} className="label-tag">
                  <TagSmallIcon />
                  {name}
                </span>
              ))}
            </div>
          ) : null}
        </div>
        <div className="task-actions">
          <button
            type="button"
            className="icon-btn"
            title="Add sub-task"
            onClick={() => nav.openAdding(task.id)}
          >
            <SubtaskIcon />
          </button>
          {recurring ? (
            <button type="button" className="icon-btn" title="Streak" onClick={() => setPanel((p) => !p)}>
              🔥
            </button>
          ) : null}
          <button
            type="button"
            className="icon-btn"
            title="Delete task"
            onClick={() => {
              deleteTask(task.id).then(onChanged, onChanged);
            }}
          >
            <TrashIcon />
          </button>
        </div>
        {progress !== null || task.streak_enabled || location !== null ? (
          <div className="task-side">
            <div className="task-badges">
              {progress !== null ? <SubtaskCount progress={progress} /> : null}
              {task.streak_enabled ? (
                <span className="streak" title="Current streak">{`🔥 ${task.streak_current}`}</span>
              ) : null}
            </div>
            {location !== null ? <span className="task-project">{location}</span> : null}
          </div>
        ) : null}
      </div>
      {panel && recurring ? <StreakPanel task={task} onChanged={onChanged} /> : null}
    </div>
  );
}
