// The Todoist-style task editor card, shared by "+ Add task", "Add sub-task"
// and editing (crates/ui/src/editor.rs `TaskEditor`). When adding, the title
// is parsed as quick add with live highlighting and preview chips. Every
// decision about the values is keel-core's (`editorView`, `applyPick`, …).
import { type MouseEvent, type ReactNode, type RefObject, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { holdPage } from "../app/pageHold";
import type { EditorPick, EditorValues, Label, Project, Recurrence, Section, SpanKind } from "../bindings";
import { localToday } from "../clock";
import { applyPick, editorView, locationPick, toggleLabel, unhighlightAt, withTitle } from "../core";
import { ArrowUpIcon, FlagFilledIcon, RepeatSmallIcon } from "../shell/icons";
import { DatePicker } from "./DatePicker";
import { DeadlinePicker } from "./DeadlinePicker";
import { HighlightedTitle } from "./HighlightedTitle";
import { flagClass, locationFace, locationItems, PRIORITY_ITEMS } from "./items";
import { useKeyboard } from "./keyboardAnchor";
import { LabelPicker } from "./LabelPicker";
import { isTouch } from "./locale";
import { Menu, type MenuItem } from "./Menu";
import { usePillFade } from "./pillFade";
import { cx, type Pop } from "./pop";
import { ReminderPicker } from "./ReminderPicker";

/** The space between the task card and the keyboard. */
const CARD_GAP = 8;

/** Stops a press taking focus from the title, except in a field of its own. */
function keepFocus(e: MouseEvent<HTMLElement>): void {
  const target = e.target as HTMLElement;
  if (!target.closest("input, textarea, select")) e.preventDefault();
}

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
  const [touch] = useState(isTouch);
  const keyboard = useKeyboard(touch);
  useEffect(() => (touch ? holdPage() : undefined), [touch]);
  const ownTitle = useRef<HTMLTextAreaElement>(null);
  const title = titleRef ?? ownTitle;
  // A calendar needs more room than the keyboard leaves, so it lowers the
  // keyboard; whatever replaces it brings the keyboard back, inside the tap.
  const madeRoom = useRef(false);
  const beforeCalendar = useRef(initial);
  const makeRoom = () => {
    // What the calendar's X goes back to.
    beforeCalendar.current = latest.current;
    if (!touch) return;
    madeRoom.current = true;
    title.current?.blur();
  };
  const setPop = (p: Pop) => {
    setOpen(p);
    if (p === "deadline" && open !== "deadline") makeRoom();
    else if (madeRoom.current) {
      madeRoom.current = false;
      title.current?.focus();
    }
  };
  const pillRow = usePillFade(values);
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
      setPop(null);
    } else {
      submitted.current = true;
    }
    onSubmit(v);
  };

  const repeatItems: MenuItem[] = view.repeat_options.map(([value, label]) => ({ value, label }));

  const calendarActions = {
    onDone: () => setPop(null),
    onCancel: () => {
      latest.current = beforeCalendar.current;
      setValues(beforeCalendar.current);
      setPop(null);
    },
  };

  const pills: ReactNode = (
    <div ref={pillRow.ref} className="chips" onScroll={pillRow.onScroll}>
      <DatePicker
        date={values.date}
        time={values.time}
        text={view.date_chip[0]}
        bucket={view.date_chip[1]}
        rule={view.repeat_rule}
        repeat={{
          items: repeatItems,
          selected: values.repeat,
          text: repeatItems.find((i) => i.value === values.repeat)?.label ?? view.repeat_chip_text,
          onSelect: (value) => pick({ type: "repeat", value }),
        }}
        today={today}
        open={open}
        setOpen={setPop}
        onChange={(date, time) => pick({ type: "date", date, time })}
        onCalendar={makeRoom}
        actions={calendarActions}
      />
      <DeadlinePicker
        deadline={values.deadline}
        text={view.deadline_text}
        urgent={view.deadline_urgent}
        today={today}
        open={open}
        setOpen={setPop}
        onChange={(date) => pick({ type: "deadline", date })}
        actions={calendarActions}
      />
      <Menu
        id="priority"
        title="Priority"
        icon={<FlagFilledIcon />}
        iconClass={flagClass(values.priority)}
        text={view.priority_text}
        items={PRIORITY_ITEMS}
        selected={values.priority}
        open={open}
        setOpen={setPop}
        chipClass={["1", "2", "3"].includes(values.priority) ? `pill-p${values.priority} pill-tinted` : ""}
        onSelect={(value) => pick({ type: "priority", value })}
      />
      <Menu
        id="location"
        title="Project / section"
        icon={locationFace(view.locations, view.location).icon}
        iconClass={locationFace(view.locations, view.location).iconClass}
        text={view.location_text}
        items={locationItems(view.locations)}
        selected={view.location}
        open={open}
        setOpen={setPop}
        onSelect={(value) => pick(locationPick(value))}
      />
      <LabelPicker
        labels={labels}
        selected={view.shown_labels}
        text={view.labels_text}
        open={open}
        setOpen={setPop}
        onToggle={(name) => setValues((v) => toggleLabel(v, name, quickAdd, projects, sections, today))}
        onChanged={onChanged}
      />
      <ReminderPicker
        reminders={values.reminders}
        labels={view.reminder_labels}
        text={view.reminders_text}
        hasDue={view.has_due}
        today={today}
        open={open}
        setOpen={setPop}
        onChange={(drafts) => pick({ type: "reminders", drafts })}
      />
      <Menu
        id="repeat"
        title="Repeat"
        icon={<RepeatSmallIcon />}
        text={view.repeat_chip_text}
        items={repeatItems}
        selected={values.repeat}
        open={open}
        setOpen={setPop}
        onSelect={(value) => pick({ type: "repeat", value })}
      />
    </div>
  );

  const clickAway = (
    // biome-ignore lint/a11y/useKeyWithClickEvents: Escape closes the popover from the form; this is the click-away.
    // biome-ignore lint/a11y/noStaticElementInteractions: a click-away layer, as in the Dioxus editor.
    <div
      className="pop-backdrop"
      onClick={() => setPop(null)}
      onWheel={() => setPop(null)}
      onMouseDown={touch ? keepFocus : undefined}
    />
  );

  const form = (
    <form
      className={cx("editor", touch && "on-keyboard", touch && !keyboard.up && "keyboard-down")}
      style={touch ? { bottom: keyboard.inset + CARD_GAP } : undefined}
      // Pressing anything but a text field keeps the title focused, so the keyboard stays up.
      onMouseDown={touch ? keepFocus : undefined}
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
      onKeyDown={(e) => {
        if (e.key !== "Escape") return;
        e.preventDefault();
        if (open !== null) setPop(null);
        else onCancel();
      }}
    >
      {!touch && open !== null ? clickAway : null}
      <HighlightedTitle
        segments={view.segments}
        value={values.content}
        placeholder={quickAdd ? "Task name — try “Call mom tomorrow 5pm p1 #Home @phone”" : "Task name"}
        onInput={(text) => setValues((v) => withTitle(v, text))}
        onEnter={submit}
        onUnhighlight={(caret) =>
          setValues((v) => (quickAdd ? (unhighlightAt(v, projects, sections, today, caret) ?? v) : v))
        }
        inputRef={title}
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
      {touch ? (
        <div className={cx("editor-actions", pillRow.fadeClass)}>
          {pills}
          <button type="submit" className="btn-save" aria-label={submitText} disabled={!view.can_submit}>
            <ArrowUpIcon />
          </button>
        </div>
      ) : (
        pills
      )}
      {touch ? null : (
        <div className="editor-footer">
          <button type="button" className="btn btn-cancel" onClick={onCancel}>
            Cancel
          </button>
          <button type="submit" className="btn btn-add" disabled={!view.can_submit}>
            {submitText}
          </button>
        </div>
      )}
    </form>
  );
  // On touch it leaves the page: a moving sheet or pane above it would carry a fixed card along.
  if (!touch) return form;
  // The dim layer sits beside the card: inside it, it would dim the card's own color.
  return createPortal(
    <>
      {open !== null ? (
        clickAway
      ) : (
        // biome-ignore lint/a11y/useKeyWithClickEvents: Escape cancels from the form; this is the tap outside.
        // biome-ignore lint/a11y/noStaticElementInteractions: a dimmed tap-away layer behind the card.
        <div className="editor-scrim" onClick={onCancel} onMouseDown={keepFocus} />
      )}
      {form}
    </>,
    document.body,
  );
}
