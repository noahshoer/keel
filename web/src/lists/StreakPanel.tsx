// A repeating task's streak: current and best, a goal, and tracking on or
// off (crates/ui/src/task_row.rs `StreakPanel`).
import type { Task } from "../bindings";
import { setStreak } from "../ipc/actions";

export function StreakPanel({ task, onChanged }: { task: Task; onChanged: () => void }) {
  const on = task.streak_enabled;
  const goal = task.streak_goal;
  const save = (enabled: boolean, g: number | null) => {
    setStreak(task.id, enabled, g).then(onChanged, onChanged);
  };
  const saveGoal = (text: string) => {
    const t = text.trim();
    // Whole numbers only (Rust's `parse::<i64>`); anything else clears the goal.
    const n = /^\d+$/.test(t) ? Number(t) : 0;
    const next = n > 0 ? n : null;
    if (next !== goal) save(true, next);
  };
  return (
    <div className="streak-panel">
      {on ? (
        <>
          <span className="stat">
            Current <b>{`🔥 ${task.streak_current}`}</b>
          </span>
          <span className="stat">
            Best <b>{task.streak_best}</b>
          </span>
          <label>
            Goal{" "}
            <input
              type="number"
              min="1"
              defaultValue={goal === null ? "" : String(goal)}
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
      <button type="button" className={on ? "btn btn-cancel" : "btn btn-add"} onClick={() => save(!on, goal)}>
        {on ? "Stop tracking" : "Track streak"}
      </button>
    </div>
  );
}
