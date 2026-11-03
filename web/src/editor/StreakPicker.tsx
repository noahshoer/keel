// The Streak pill, offered once a task being added repeats, and its popover:
// a goal, and the way back out. A tap turns the streak on, so tapping away
// leaves it on with no goal.
import { type RefObject, useRef } from "react";
import { openAndFocus } from "../shell/focus";
import { ChevronDownSmallIcon, FireIcon, TargetSmallIcon, TrashSmallIcon } from "../shell/icons";
import { cx, type PopProps } from "./pop";
import { usePlacedPopover } from "./popover";

/** More than anyone's goal, and short enough to stay a safe integer. */
const MAX_DIGITS = 5;

function GoalPopover({
  goal,
  field,
  onGoal,
  onRemove,
  onEnter,
}: {
  goal: number | null;
  field: RefObject<HTMLInputElement | null>;
  onGoal: (goal: number | null) => void;
  onRemove: () => void;
  onEnter: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  return (
    <div ref={ref} className="popover menu streak-goal-pop">
      <label className="streak-goal-row">
        <span className="streak-goal-icon">
          <TargetSmallIcon />
        </span>
        <span className="menu-label">Goal</span>
        <input
          ref={field}
          type="text"
          inputMode="numeric"
          // With this pattern iOS opens the 0-9 pad, not the symbols keyboard.
          pattern="[0-9]*"
          placeholder="None"
          value={goal === null ? "" : String(goal)}
          onChange={(e) => {
            const digits = e.target.value.replace(/\D/g, "").slice(0, MAX_DIGITS);
            onGoal(digits === "" ? null : Number(digits));
          }}
          onKeyDown={(e) => {
            if (e.key !== "Enter") return;
            // Not the task's Enter: it closes this, and the task saves from the title.
            e.preventDefault();
            onEnter();
          }}
        />
      </label>
      <button
        type="button"
        className="icon-btn streak-goal-remove"
        title="Remove streak"
        aria-label="Remove streak"
        onClick={onRemove}
      >
        <TrashSmallIcon />
      </button>
    </div>
  );
}

export function StreakPicker({
  text,
  on,
  goal,
  open,
  setOpen,
  onPick,
  onEnter,
}: PopProps & {
  text: string;
  on: boolean;
  goal: number | null;
  onPick: (on: boolean, goal: number | null) => void;
  /** Enter in the goal: after it closes, the title gets the keyboard. */
  onEnter: () => void;
}) {
  const isOpen = open === "streak-goal";
  const field = useRef<HTMLInputElement>(null);
  return (
    <div className="chip-wrap">
      <button
        type="button"
        className={cx("chip chip-btn pill-streak", on ? "pill-set" : "pill-offer", isOpen && "chip-open")}
        title="Streak"
        aria-pressed={on}
        onClick={() => {
          if (isOpen) {
            setOpen(null);
            return;
          }
          openAndFocus(() => {
            // A typed streak is already on, and keeps its words until its goal changes.
            if (!on) onPick(true, null);
            setOpen("streak-goal");
          }, field);
        }}
      >
        <span className="chip-icon">
          <FireIcon size={12} />
        </span>
        {text}
        {on ? (
          <span className="chip-caret">
            <ChevronDownSmallIcon />
          </span>
        ) : null}
      </button>
      {isOpen ? (
        <GoalPopover
          goal={goal}
          field={field}
          onGoal={(g) => onPick(true, g)}
          onRemove={() => {
            onPick(false, null);
            setOpen(null);
          }}
          onEnter={onEnter}
        />
      ) : null}
    </div>
  );
}
