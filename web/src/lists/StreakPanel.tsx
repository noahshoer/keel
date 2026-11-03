// A repeating task's streak: current and best, a goal, and tracking on or
// off (crates/ui/src/task_row.rs `StreakPanel`).

import { useEffect, useRef } from "react";
import type { Task } from "../bindings";
import { setStreak } from "../ipc/actions";
import { FireIcon } from "../shell/icons";

export function StreakPanel({ task, onChanged }: { task: Task; onChanged: () => void }) {
  const on = task.streak_enabled;
  const goal = task.streak_goal;
  const save = (enabled: boolean, g: number | null) => {
    setStreak(task.id, enabled, g).then(onChanged, onChanged);
  };
  // What's typed and not yet saved. A popover can close around the field
  // without it being left, and then there's no blur.
  const typed = useRef<string | null>(null);
  const saveGoal = (text: string) => {
    typed.current = null;
    const t = text.trim();
    // Whole numbers only (Rust's `parse::<i64>`); anything else clears the goal.
    const n = /^\d+$/.test(t) ? Number(t) : 0;
    const next = n > 0 ? n : null;
    if (next !== goal) save(true, next);
  };
  const saveLatest = useRef(saveGoal);
  saveLatest.current = saveGoal;
  useEffect(
    () => () => {
      if (typed.current !== null) saveLatest.current(typed.current);
    },
    [],
  );
  return (
    <div className="streak-panel">
      {on ? (
        <>
          <span className="stat">
            Current{" "}
            <b className="streak-count">
              <FireIcon />
              {task.streak_current}
            </b>
          </span>
          <span className="stat">
            Best <b>{task.streak_best}</b>
          </span>
          <label className="stat">
            Goal{" "}
            <input
              type="text"
              inputMode="numeric"
              // With this pattern iOS opens the 0-9 pad, not the symbols keyboard.
              pattern="[0-9]*"
              defaultValue={goal === null ? "" : String(goal)}
              // A number field still lets WebKit type letters and pastes.
              onChange={(e) => {
                e.target.value = e.target.value.replace(/\D/g, "");
                typed.current = e.target.value;
              }}
              // Saved when the field is left or on Enter (the DOM change
              // event, as in Dioxus), not on every keystroke.
              onBlur={(e) => saveGoal(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") saveGoal(e.currentTarget.value);
              }}
            />
          </label>
        </>
      ) : (
        <span>Track how many times in a row you complete this.</span>
      )}
      <button
        type="button"
        className={on ? "btn btn-cancel" : "btn btn-add"}
        onClick={() => {
          typed.current = null;
          save(!on, goal);
        }}
      >
        {on ? "Stop tracking" : "Track streak"}
      </button>
    </div>
  );
}
