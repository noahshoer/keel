// The Date chip and its scheduler (crates/ui/src/pickers.rs `DatePicker`).
import type { DueBucket, Recurrence } from "../bindings";
import { BUCKET_CLASS } from "../lists/chips";
import { CalendarSmallIcon, RepeatSmallIcon } from "../shell/icons";
import { cx, type PopProps } from "./pop";
import { Scheduler } from "./Scheduler";

export function DatePicker({
  date,
  time,
  text,
  bucket,
  rule,
  today,
  open,
  setOpen,
  onChange,
}: PopProps & {
  /** `YYYY-MM-DD` or "". */
  date: string;
  /** `HH:MM` or "". */
  time: string;
  /** The chip's text and colour (`EditorView.date_chip`). */
  text: string;
  bucket: DueBucket | null;
  /** The repeat rule: a ⟳ on the chip and ringed days in the calendar. */
  rule: Recurrence | null;
  today: string;
  /** The date (`YYYY-MM-DD` or "") and time (`HH:MM` or ""). */
  onChange: (date: string, time: string) => void;
}) {
  const isOpen = open === "date";
  return (
    <div className="chip-wrap">
      <button
        type="button"
        className={cx(
          "chip chip-btn due-chip",
          bucket !== null ? BUCKET_CLASS[bucket] : "",
          isOpen && "chip-open",
        )}
        title="Date"
        onClick={() => setOpen(isOpen ? null : "date")}
      >
        <CalendarSmallIcon />
        {text}
        {rule !== null ? <RepeatSmallIcon /> : null}
      </button>
      {isOpen ? (
        <Scheduler
          selected={date === "" ? null : date}
          time={time === "" ? null : time}
          rule={rule}
          today={today}
          onPick={(d, t) => onChange(d ?? "", t)}
          close={() => setOpen(null)}
        />
      ) : null}
    </div>
  );
}
