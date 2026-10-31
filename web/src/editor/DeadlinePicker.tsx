// The Deadline chip and its date-only scheduler
// (crates/ui/src/pickers.rs `DeadlinePicker`).
import { TargetSmallIcon } from "../shell/icons";
import { cx, type Face, type PopProps } from "./pop";
import { Scheduler } from "./Scheduler";

export function DeadlinePicker({
  deadline,
  text,
  urgent,
  today,
  open,
  setOpen,
  onChange,
  face,
}: PopProps & {
  /** `YYYY-MM-DD` or "". */
  deadline: string;
  /** The chip's text. */
  text: string;
  /** `EditorView.deadline_urgent`: `null` without a deadline. */
  urgent: boolean | null;
  today: string;
  /** `YYYY-MM-DD`, or "" to clear. */
  onChange: (date: string) => void;
  /** The task view's look for the button. */
  face?: Face;
}) {
  const isOpen = open === "deadline";
  const state = urgent === null ? "deadline-chip" : urgent ? "deadline-chip urgent" : "deadline-chip set";
  return (
    <div className="chip-wrap">
      <button
        type="button"
        className={cx(face?.className ?? `chip chip-btn ${state}`, isOpen && "chip-open")}
        title="Deadline: the date it must be done by"
        onClick={() => setOpen(isOpen ? null : "deadline")}
      >
        {face === undefined || face.icon ? <TargetSmallIcon /> : null}
        {face?.text ?? text}
      </button>
      {isOpen ? (
        <Scheduler
          selected={deadline === "" ? null : deadline}
          time={null}
          withTime={false}
          emptyText="No deadline"
          rule={null}
          today={today}
          onPick={(d) => onChange(d ?? "")}
          close={() => setOpen(null)}
        />
      ) : null}
    </div>
  );
}
