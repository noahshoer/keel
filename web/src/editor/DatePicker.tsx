// The Date chip and its scheduler (crates/ui/src/pickers.rs `DatePicker`).

import type { ReactNode } from "react";
import type { DueBucket, Recurrence } from "../bindings";
import { BUCKET_CLASS } from "../lists/chips";
import { CalendarSmallIcon, RepeatSmallIcon } from "../shell/icons";
import { cx, type Face, type PopProps } from "./pop";
import { type CalendarActions, type RepeatChoice, Scheduler } from "./Scheduler";

export function DatePicker({
  date,
  time,
  text,
  bucket,
  rule,
  repeat,
  onCalendar,
  actions,
  today,
  open,
  setOpen,
  onChange,
  face,
  icon,
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
  /** The date card's Repeat row and options. */
  repeat?: RepeatChoice;
  /** When the quick choices give way to the calendar. */
  onCalendar?: () => void;
  /** The calendar's check and X. */
  actions?: CalendarActions;
  today: string;
  /** The date (`YYYY-MM-DD` or "") and time (`HH:MM` or ""). */
  onChange: (date: string, time: string) => void;
  /** The task view's look for the button. */
  face?: Face;
  /** The button's icon in place of the calendar outline. */
  icon?: ReactNode;
}) {
  const isOpen = open === "date";
  return (
    <div className="chip-wrap">
      <button
        type="button"
        className={cx(
          face?.className ?? cx("chip chip-btn due-chip", bucket !== null ? BUCKET_CLASS[bucket] : ""),
          isOpen && "chip-open",
        )}
        title="Date"
        onClick={() => setOpen(isOpen ? null : "date")}
      >
        {face === undefined || face.icon ? (icon ?? <CalendarSmallIcon />) : null}
        {face === undefined ? (
          <>
            {text}
            {rule !== null ? <RepeatSmallIcon /> : null}
          </>
        ) : (
          face.text
        )}
      </button>
      {isOpen ? (
        <Scheduler
          selected={date === "" ? null : date}
          time={time === "" ? null : time}
          rule={rule}
          repeat={repeat}
          onCalendar={onCalendar}
          actions={actions}
          today={today}
          onPick={(d, t) => onChange(d ?? "", t)}
          close={() => setOpen(null)}
        />
      ) : null}
    </div>
  );
}
