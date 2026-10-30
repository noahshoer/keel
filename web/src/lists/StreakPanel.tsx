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
              onChange={(e) => {
                const n = Number.parseInt(e.target.value.trim(), 10);
                save(true, Number.isFinite(n) && n > 0 ? n : null);
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
