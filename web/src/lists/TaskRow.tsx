// One task in a list: the markup and behavior of crates/ui/src/task_row.rs
// `TaskRow` without its glass parts (Liquid Glass is parked). Tapping the
// circle completes the task after the fill and switch-off; the body opens
// the task view, and Add sub-task opens it adding.
import { type CSSProperties, Fragment, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { CardRow, Label, LabelGroup, Reminder } from "../bindings";
import { deadlineUrgent, dueLabel, labelGroups, reminderLabel } from "../core";
import { labelColorVar } from "../editor/labelColors";
import { completeTask, deleteTask } from "../ipc/actions";
import {
  BellRingingIcon,
  CalendarSmallIcon,
  DoneMark,
  FireIcon,
  FlagFillIcon,
  RepeatSmallIcon,
  SubtaskIcon,
  TagSmallIcon,
  TrashIcon,
} from "../shell/icons";
import { useTaskNav } from "../taskview/nav-context";
import { CHECK_CLASS, dueChip } from "./chips";
import { CompletionIconContext, FILL_MS, nextOnTap, OFF_MS, type Phase, sleep } from "./completion";
import { bottomLayout, type RowLocation } from "./rowLayout";
import { StreakPanel } from "./StreakPanel";
import { SubtaskProgress } from "./SubtaskProgress";
import { useHiddenCount } from "./useHiddenCount";

/** The labels on a row: one tag per color, then "+N" for any cut off. */
function RowLabels({ groups }: { groups: LabelGroup[] }) {
  const ref = useRef<HTMLDivElement>(null);
  const hidden = useHiddenCount(ref, groups.flatMap((g) => g.names).join("\n"));
  return (
    <div className="task-labels row-shrink">
      <div ref={ref} className="task-label-items">
        {groups.map((group) => (
          <span
            key={group.color}
            className="task-label-group"
            style={{ "--tint": labelColorVar(group.color) } as CSSProperties}
          >
            <TagSmallIcon />
            {group.names.map((name, i) => (
              <Fragment key={name}>
                {i > 0 ? ", " : null}
                <span data-label="">{name}</span>
              </Fragment>
            ))}
          </span>
        ))}
      </div>
      {hidden > 0 ? <span className="label-more">{`+${hidden}`}</span> : null}
    </div>
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
  labels,
  reminders,
  onChanged,
}: {
  row: CardRow;
  today: string;
  /** The task's project and section, on every row. */
  location: RowLocation;
  /** Every label; a task's are grouped by their colors. */
  labels: Label[];
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
  // A wasm call serializes both lists, so only when there's something to group.
  const groups = useMemo(
    () => (task.labels.length > 0 ? labelGroups(task.labels, labels) : []),
    [task.labels, labels],
  );
  const hasWhen = progress !== null || due !== null || mine.length > 0;
  const whenGroup = hasWhen ? (
    <div className="task-when task-badges">
      {progress !== null ? <SubtaskProgress progress={progress} /> : null}
      {due !== null ? (
        <span className={due.cls}>
          <CalendarSmallIcon />
          {due.label}
        </span>
      ) : null}
      {recurring ? (
        <span className="task-repeat" title="Repeats">
          <RepeatSmallIcon />
        </span>
      ) : null}
      {mine.length > 0 ? (
        <span className="reminder-tag" title={reminderTitle}>
          <BellRingingIcon />
        </span>
      ) : null}
    </div>
  ) : null;
  const labelRow = groups.length > 0 ? <RowLabels groups={groups} /> : null;
  const project = (
    <span className="task-project row-shrink">
      <span className="task-project-name">{location.project}</span>
      {location.section !== null ? (
        <span className="task-project-section">{` / ${location.section}`}</span>
      ) : null}
    </span>
  );

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
            <span className="task-content">{task.content}</span>
            {task.streak_enabled ? (
              <span className="streak" title="Current streak">
                <FireIcon />
                {task.streak_current}
              </span>
            ) : null}
          </div>
          {deadline !== null ? (
            <div className="task-deadline">
              <span className={`deadline ${deadline.cls}`} title="Deadline">
                <FlagFillIcon />
                {deadline.label}
              </span>
            </div>
          ) : null}
          {task.description ? <div className="task-desc">{task.description}</div> : null}
          {bottomLayout(hasWhen, labelRow !== null) === "two-rows" ? (
            <>
              {whenGroup}
              <div className="task-bottom">
                {labelRow}
                {project}
              </div>
            </>
          ) : (
            <div className="task-bottom">
              {whenGroup ?? labelRow}
              {project}
            </div>
          )}
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
              <FireIcon />
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
      </div>
      {panel && recurring ? <StreakPanel task={task} onChanged={onChanged} /> : null}
    </div>
  );
}
