// One field's control in the task view: the editor's own picker with the
// view's face, a row's or a dotted pill's (crates/ui/src/task_view/view.rs,
// the non-native branch of the `field` closure). Every text and class comes
// from the one `editorView` the view draws from.
import type { EditorPick, EditorValues, EditorView, Field, Label, MissingPill } from "../bindings";
import { locationPick } from "../core";
import { DatePicker } from "../editor/DatePicker";
import { DeadlinePicker } from "../editor/DeadlinePicker";
import { flagClass, locationItems, PRIORITY_ITEMS } from "../editor/items";
import { LabelPicker } from "../editor/LabelPicker";
import { Menu, type MenuItem } from "../editor/Menu";
import type { Face, PopProps } from "../editor/pop";
import { ReminderPicker } from "../editor/ReminderPicker";
import { BUCKET_CLASS } from "../lists/chips";
import { CalendarFilledIcon, FlagFilledIcon, HashSmallIcon, RepeatSmallIcon } from "../shell/icons";

const rowFace = (cls: string, text: string): Face => ({
  className: cls === "" ? "tv-field-btn" : `tv-field-btn ${cls}`,
  text,
  icon: true,
});

const pillFace = (m: MissingPill): Face => ({ className: "tv-pill", text: m.label, icon: false });

export function FieldControl({
  field,
  missing,
  values: v,
  view,
  labels,
  today,
  open,
  setOpen,
  onPick,
  onToggle,
  onLabelsChanged,
  onOpenDescription,
}: PopProps & {
  /** A set field's row (`missing` is null), or… */
  field: Field | null;
  /** …a missing one's pill (`field` is null). */
  missing: MissingPill | null;
  values: EditorValues;
  view: EditorView;
  labels: Label[];
  today: string;
  onPick: (p: EditorPick) => void;
  onToggle: (name: string) => void;
  /** After a label was created from the labels menu. */
  onLabelsChanged: () => void;
  /** "+ Description": opens an empty description row. */
  onOpenDescription: () => void;
}) {
  const row = field !== null;
  const which = field ?? (missing?.missing.kind === "Field" ? missing.missing.field : null);
  const face = (cls: string, text: string) => (missing !== null ? pillFace(missing) : rowFace(cls, text));
  const pop = { open, setOpen };
  const repeatItems: MenuItem[] = view.repeat_options.map(([value, label]) => ({ value, label }));
  const repeat = (f: Face) => (
    <Menu
      id="repeat"
      title="Repeat"
      icon={<RepeatSmallIcon />}
      text={view.repeat_chip_text}
      items={repeatItems}
      selected={v.repeat}
      {...pop}
      face={f}
      onSelect={(value) => onPick({ type: "repeat", value })}
    />
  );

  if (missing?.missing.kind === "Repeat") return repeat(pillFace(missing));
  switch (which) {
    case "Description":
      // A set description is typed in place (the view's row); only its pill gets here.
      return missing !== null ? (
        <button type="button" className="tv-pill" onClick={onOpenDescription}>
          {missing.label}
        </button>
      ) : null;
    case "Date": {
      const [text, bucket] = view.date_chip;
      return (
        <>
          <DatePicker
            date={v.date}
            time={v.time}
            text={text}
            bucket={bucket}
            rule={view.repeat_rule}
            today={today}
            {...pop}
            face={face(`due-chip ${bucket !== null ? BUCKET_CLASS[bucket] : ""}`.trim(), text)}
            icon={<CalendarFilledIcon />}
            onChange={(date, time) => onPick({ type: "date", date, time })}
          />
          {/* A rule shows in the date's row; without one it's a pill. */}
          {row && v.repeat !== "none"
            ? repeat({ className: "tv-field-btn tv-repeat", text: view.repeat_chip_text, icon: true })
            : null}
        </>
      );
    }
    case "Deadline":
      return (
        <DeadlinePicker
          deadline={v.deadline}
          text={view.deadline_text}
          urgent={view.deadline_urgent}
          today={today}
          {...pop}
          face={face(
            view.deadline_urgent === true ? "deadline-chip urgent" : "deadline-chip set",
            `Deadline ${view.deadline_text}`,
          )}
          onChange={(date) => onPick({ type: "deadline", date })}
        />
      );
    case "Priority":
      return (
        <Menu
          id="priority"
          title="Priority"
          icon={<FlagFilledIcon />}
          iconClass={flagClass(v.priority)}
          text=""
          items={PRIORITY_ITEMS}
          selected={v.priority}
          {...pop}
          face={face("", `Priority ${v.priority}`)}
          onSelect={(value) => onPick({ type: "priority", value })}
        />
      );
    case "Labels":
      return (
        <LabelPicker
          labels={labels}
          selected={v.labels}
          text=""
          {...pop}
          face={face("", v.labels.join(", "))}
          onToggle={onToggle}
          onChanged={onLabelsChanged}
        />
      );
    case "Reminders":
      return (
        <ReminderPicker
          reminders={v.reminders}
          labels={view.reminder_labels}
          text=""
          hasDue={view.has_due}
          today={today}
          {...pop}
          face={face("", view.reminders_text)}
          onChange={(drafts) => onPick({ type: "reminders", drafts })}
        />
      );
    case "Location":
      return (
        <Menu
          id="location"
          title="Project / section"
          icon={<HashSmallIcon />}
          text=""
          items={locationItems(view.locations)}
          selected={view.location}
          {...pop}
          face={face("", view.location_text)}
          onSelect={(value) => onPick(locationPick(value))}
        />
      );
    case null:
      return null;
  }
}
