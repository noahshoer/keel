// The reminders chip and popover: current reminders (removable), presets
// relative to the due time, and a custom time typed in plain words
// (crates/ui/src/pickers.rs `ReminderPicker`).
import { useRef, useState } from "react";
import type { ReminderDraft } from "../bindings";
import { customReminderAt, reminderPresets } from "../core";
import { BellSmallIcon, PlusIcon } from "../shell/icons";
import { cx, type PopProps } from "./pop";
import { usePlacedPopover } from "./popover";

function ReminderMenu({
  reminders,
  labels,
  hasDue,
  today,
  onChange,
}: {
  reminders: ReminderDraft[];
  labels: string[];
  hasDue: boolean;
  today: string;
  onChange: (drafts: ReminderDraft[]) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  const [custom, setCustom] = useState("");
  const [bad, setBad] = useState(false);
  const add = (draft: ReminderDraft) => {
    const dup = reminders.some((r) => r.minuteOffset === draft.minuteOffset && r.at === draft.at);
    onChange(dup ? reminders : [...reminders, draft]);
  };
  return (
    <div ref={ref} className="popover menu reminder-pop">
      {reminders.length === 0 ? <div className="label-empty">No reminders yet</div> : null}
      {reminders.map((_, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: new drafts have no id; the list is positional, as in Dioxus.
        <div key={i} className="reminder-item">
          <span className="menu-icon">
            <BellSmallIcon />
          </span>
          <span className="menu-label">{labels[i] ?? ""}</span>
          <button
            type="button"
            className="icon-btn"
            title="Remove reminder"
            onClick={() => onChange(reminders.filter((_, j) => j !== i))}
          >
            ×
          </button>
        </div>
      ))}
      <div className="menu-divider" />
      {!hasDue ? <div className="label-empty">Give the task a date to use these:</div> : null}
      {reminderPresets().map(([m, text]) => (
        <button
          key={`p${m}`}
          type="button"
          className="menu-item"
          disabled={!hasDue}
          onClick={() => add({ id: null, minuteOffset: m, at: null })}
        >
          <span className="menu-icon">
            <PlusIcon size={14} />
          </span>
          <span className="menu-label">{text}</span>
        </button>
      ))}
      <input
        className={bad ? "label-new bad" : "label-new"}
        placeholder="Custom, e.g. tomorrow 9am, then Enter"
        value={custom}
        onChange={(e) => {
          setBad(false);
          setCustom(e.target.value);
        }}
        onKeyDown={(e) => {
          if (e.key !== "Enter") return;
          e.preventDefault();
          e.stopPropagation();
          const at = customReminderAt(custom, today);
          if (at === null) {
            setBad(true);
            return;
          }
          add({ id: null, minuteOffset: null, at });
          setCustom("");
          setBad(false);
        }}
      />
      {bad ? (
        <div className="filter-problem">Needs a time, like “friday 3pm” or “in 2 days at 9am”.</div>
      ) : null}
    </div>
  );
}

export function ReminderPicker({
  reminders,
  labels,
  text,
  hasDue,
  today,
  open,
  setOpen,
  onChange,
}: PopProps & {
  reminders: ReminderDraft[];
  /** Each reminder's text (`EditorView.reminder_labels`). */
  labels: string[];
  /** The chip's text. */
  text: string;
  /** Whether the task has a due date (relative presets need one). */
  hasDue: boolean;
  today: string;
  onChange: (drafts: ReminderDraft[]) => void;
}) {
  const isOpen = open === "reminders";
  return (
    <div className="chip-wrap">
      <button
        type="button"
        className={cx("chip chip-btn", reminders.length > 0 && "reminder-chip", isOpen && "chip-open")}
        title="Reminders"
        onClick={() => setOpen(isOpen ? null : "reminders")}
      >
        <BellSmallIcon />
        {text}
      </button>
      {isOpen ? (
        <ReminderMenu
          reminders={reminders}
          labels={labels}
          hasDue={hasDue}
          today={today}
          onChange={onChange}
        />
      ) : null}
    </div>
  );
}
