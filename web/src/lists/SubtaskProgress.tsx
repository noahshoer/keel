// A task's sub-tasks: an icon for how far along they are, then "X of Y".
import type { Progress } from "../bindings";
import { progressLong } from "../core";
import { CheckCircleIcon, CircleDashedIcon } from "../shell/icons";
import { ringDash, subtaskState } from "./rowLayout";

function Ring({ progress }: { progress: Progress }) {
  return (
    <span className="icon">
      <svg className="subtask-ring" viewBox="0 0 256 256" fill="none" aria-hidden="true">
        <circle cx="128" cy="128" r="96" className="subtask-ring-track" strokeWidth="24" />
        <circle
          cx="128"
          cy="128"
          r="96"
          className="subtask-ring-fill"
          strokeWidth="24"
          strokeLinecap="round"
          strokeDasharray={ringDash(progress)}
          transform="rotate(-90 128 128)"
        />
      </svg>
    </span>
  );
}

export function SubtaskProgress({ progress }: { progress: Progress }) {
  const state = subtaskState(progress);
  return (
    <span className="task-subtasks">
      <span className="subtask-icon" data-progress={state}>
        {state === "none" ? <CircleDashedIcon /> : null}
        {state === "some" ? <Ring progress={progress} /> : null}
        {state === "all" ? <CheckCircleIcon /> : null}
      </span>
      <span className="task-count">{progressLong(progress)}</span>
    </span>
  );
}
