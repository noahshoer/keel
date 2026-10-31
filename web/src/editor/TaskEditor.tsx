// The Todoist-style task editor card, shared by "+ Add task", "Add sub-task"
// and editing (crates/ui/src/editor.rs `TaskEditor`). When adding, the title
// is parsed as quick add with live highlighting and preview chips. Every
// decision about the values is keel-core's (`editorView`, `applyPick`, …).
import { type RefObject, useRef, useState } from "react";
import type { EditorPick, EditorValues, Label, Project, Recurrence, Section, SpanKind } from "../bindings";
import { localToday } from "../clock";
import { applyPick, editorView, locationPick, toggleLabel, unhighlightAt, withTitle } from "../core";
import { FlagSmallIcon, HashSmallIcon, RepeatSmallIcon } from "../shell/icons";
import { DatePicker } from "./DatePicker";
import { DeadlinePicker } from "./DeadlinePicker";
import { HighlightedTitle } from "./HighlightedTitle";
import { flagClass, locationItems, PRIORITY_ITEMS } from "./items";
import { LabelPicker } from "./LabelPicker";
import { Menu, type MenuItem } from "./Menu";
import type { Pop } from "./pop";
import { ReminderPicker } from "./ReminderPicker";

const PREVIEW: Record<SpanKind, string> = {
  Date: "chip-date",
  Time: "chip-date",
  Repeat: "chip-repeat",
  Priority: "chip-priority",
  Project: "chip-project",
  Section: "chip-project",
  Label: "chip-label",
  Deadline: "chip-deadline",
};

export function TaskEditor({
  initial,
  projects,
  sections,
  labels,
  keepLabel,
  existing,
  submitText,
  quickAdd,
  keepOpen,
  onSubmit,
  onCancel,
  onChanged,
  titleRef,
}: {
  initial: EditorValues;
  projects: Project[];
  sections: Section[];
  labels: Label[];
  /** Text for the "keep current rule" repeat option, when the rule is custom. */
  keepLabel: string | null;
  /** The edited task's recurrence (what "keep" repeats by). */
  existing: Recurrence | null;
  submitText: string;
  /** Parse the title as quick add (adding) and preview it. */
  quickAdd: boolean;
  /** Stay open after submitting, reset to `initial` (Todoist's add flow). */
  keepOpen: boolean;
  onSubmit: (v: EditorValues) => void;
  onCancel: () => void;
  /** After a label was created from the labels menu. */
  onChanged: () => void;
  /** The title field, for a caller that focuses it inside a tap. */
  titleRef?: RefObject<HTMLTextAreaElement | null>;
}) {
  const [values, setValues] = useState(initial);
  const [open, setOpen] = useState<Pop>(null);
  // Submits once: a fast second Enter (before the editor closes) is ignored.
  // In keep-open mode the form resets to blank instead (an empty title can't
  // be submitted, so a second Enter is a no-op there too). Refs, so two
  // Enters handled before a re-render still see the first one's effect.
  const submitted = useRef(false);
  const latest = useRef(values);
  latest.current = values;
  const today = localToday();
  const view = editorView(values, quickAdd, keepLabel, existing, projects, sections, today);
  const pick = (p: EditorPick) => setValues((v) => applyPick(v, p, quickAdd, projects, sections, today));

  const submit = () => {
    const v = latest.current;
    if (
      submitted.current ||
      !editorView(v, quickAdd, keepLabel, existing, projects, sections, today).can_submit
    )
      return;
    if (keepOpen) {
      latest.current = initial;
      setValues(initial);
      setOpen(null);
    } else {
      submitted.current = true;
    }
    onSubmit(v);
  };

  const repeatItems: MenuItem[] = view.repeat_options.map(([value, label]) => ({ value, label }));

  return (
    <form
      className="editor"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
      onKeyDown={(e) => {
        if (e.key !== "Escape") return;
        e.preventDefault();
        if (open !== null) setOpen(null);
        else onCancel();
      }}
    >
      {open !== null ? (
        // biome-ignore lint/a11y/useKeyWithClickEvents: Escape closes the popover from the form; this is the click-away.
        // biome-ignore lint/a11y/noStaticElementInteractions: a transparent click-away layer, as in the Dioxus editor.
        <div className="pop-backdrop" onClick={() => setOpen(null)} onWheel={() => setOpen(null)} />
      ) : null}
      <HighlightedTitle
        segments={view.segments}
        value={values.content}
        placeholder={quickAdd ? "Task name — try “Call mom tomorrow 5pm p1 #Home @phone”" : "Task name"}
        onInput={(text) => setValues((v) => withTitle(v, text))}
        onEnter={submit}
        onUnhighlight={(caret) =>
          setValues((v) => (quickAdd ? (unhighlightAt(v, projects, sections, today, caret) ?? v) : v))
        }
        inputRef={titleRef}
      />
      {view.preview.length > 0 ? (
        <div className="quick-preview">
          {view.preview.map((c, i) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: the chips follow the parse's order.
            <span key={i} className={`quick-chip ${PREVIEW[c.kind]}`}>
              {c.text}
            </span>
          ))}
        </div>
      ) : null}
      <input
        className="desc"
        placeholder="Description"
        value={values.description}
        onChange={(e) => {
          const description = e.target.value;
          setValues((v) => ({ ...v, description }));
        }}
      />
      <div className="chips">
        <DatePicker
          date={values.date}
          time={values.time}
          text={view.date_chip[0]}
          bucket={view.date_chip[1]}
          rule={view.repeat_rule}
          today={today}
          open={open}
          setOpen={setOpen}
          onChange={(date, time) => pick({ type: "date", date, time })}
        />
        <DeadlinePicker
          deadline={values.deadline}
          text={view.deadline_text}
          urgent={view.deadline_urgent}
          today={today}
          open={open}
          setOpen={setOpen}
          onChange={(date) => pick({ type: "deadline", date })}
        />
        <ReminderPicker
          reminders={values.reminders}
          labels={view.reminder_labels}
          text={view.reminders_text}
          hasDue={view.has_due}
          today={today}
          open={open}
          setOpen={setOpen}
          onChange={(drafts) => pick({ type: "reminders", drafts })}
        />
        <Menu
          id="priority"
          title="Priority"
          icon={<FlagSmallIcon />}
          iconClass={flagClass(values.priority)}
          text={view.priority_text}
          items={PRIORITY_ITEMS}
          selected={values.priority}
          open={open}
          setOpen={setOpen}
          onSelect={(value) => pick({ type: "priority", value })}
        />
        <Menu
          id="repeat"
          title="Repeat"
          icon={<RepeatSmallIcon />}
          text={view.repeat_chip_text}
          items={repeatItems}
          selected={values.repeat}
          open={open}
          setOpen={setOpen}
          onSelect={(value) => pick({ type: "repeat", value })}
        />
        <Menu
          id="location"
          title="Project / section"
          icon={<HashSmallIcon />}
          text={view.location_text}
          items={locationItems(view.locations)}
          selected={view.location}
          open={open}
          setOpen={setOpen}
          onSelect={(value) => pick(locationPick(value))}
        />
        <LabelPicker
          labels={labels}
          selected={view.shown_labels}
          text={view.labels_text}
          open={open}
          setOpen={setOpen}
          onToggle={(name) => setValues((v) => toggleLabel(v, name, quickAdd, projects, sections, today))}
          onChanged={onChanged}
        />
      </div>
      <div className="editor-footer">
        <button type="button" className="btn btn-cancel" onClick={onCancel}>
          Cancel
        </button>
        <button type="submit" className="btn btn-add" disabled={!view.can_submit}>
          {submitText}
        </button>
      </div>
    </form>
  );
}
