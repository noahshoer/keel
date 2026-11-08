// The Todoist-style task editor card, shared by "+ Add task", "Add sub-task"
// and editing (crates/ui/src/editor.rs `TaskEditor`). When adding, the title
// is parsed as quick add with live highlighting, and what it says fills the pills. Every
// decision about the values is keel-core's (`editorView`, `applyPick`, …).
import {
  type CSSProperties,
  type MouseEvent,
  type PointerEvent,
  type ReactNode,
  type RefObject,
  useEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { holdPage } from "../app/pageHold";
import type {
  EditorPick,
  EditorValues,
  EditorView,
  Label,
  Project,
  Recurrence,
  Section,
  SpanKind,
} from "../bindings";
import { localToday } from "../clock";
import {
  applyPick,
  editorView,
  locationPick,
  readRepeat,
  toggleLabel,
  unhighlightAt,
  withTitle,
} from "../core";
import { ArrowUpIcon, FlagFilledIcon, RepeatSmallIcon } from "../shell/icons";
import { DatePicker } from "./DatePicker";
import { DeadlinePicker } from "./DeadlinePicker";
import { HighlightedTitle } from "./HighlightedTitle";
import { flagClass, locationChosen, locationFace, locationItems, PRIORITY_ITEMS } from "./items";
import { CARD_GAP, lowerKeyboardNow, useHeldKeyboard, useKeyboard } from "./keyboardAnchor";
import { LabelPicker } from "./LabelPicker";
import { isTouch } from "./locale";
import { Menu, type MenuItem } from "./Menu";
import { type TypedPills, usePillFade, useRevealFilled } from "./pillFade";
import { cx, DEADLINE_TITLE, type Pop } from "./pop";
import { ReminderPicker } from "./ReminderPicker";
import { StreakPicker } from "./StreakPicker";

/** Stops a press taking focus from the title, except in a field of its own. */
function keepFocus(e: MouseEvent<HTMLElement>): void {
  const target = e.target as HTMLElement;
  if (!target.closest("input, textarea, select")) e.preventDefault();
}

function streakPillText(view: EditorView): string {
  if (!view.shown.streak) return "+ Streak";
  return view.shown.streak_goal === null ? "Streak" : `Streak ${view.shown.streak_goal}`;
}

/** What each pill with a phrase in the title says, by the pill's `title`. */
function typedPills(view: EditorView): TypedPills {
  const pills: Record<SpanKind, [string, string]> = {
    Date: ["Date", view.date_chip[0]],
    Time: ["Date", view.date_chip[0]],
    Deadline: [DEADLINE_TITLE, view.deadline_text],
    Priority: ["Priority", view.priority_text],
    Project: ["Project / section", view.location_text],
    Section: ["Project / section", view.location_text],
    Label: ["Labels", view.labels_text],
    Repeat: ["Repeat", view.repeat_chip_text],
    Streak: ["Streak", streakPillText(view)],
  };
  const typed: TypedPills = {};
  for (const { kind } of view.segments) {
    if (kind !== null) {
      const [title, text] = pills[kind];
      typed[title] = text;
    }
  }
  // It appears with a repeat, beyond the end of the row: bring it into view.
  if (view.streak_offered) typed["Streak"] = streakPillText(view);
  return typed;
}

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
  /** Parse the title as quick add (adding), filling the pills from it. */
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
  const keyboard = useHeldKeyboard(useKeyboard(touch, true), open === "streak-goal");
  useEffect(() => (touch ? holdPage() : undefined), [touch]);
  const ownTitle = useRef<HTMLTextAreaElement>(null);
  const title = titleRef ?? ownTitle;
  // The date's calendar needs more room than the keyboard leaves, so it lowers
  // the keyboard; whatever replaces it brings the keyboard back, inside the tap.
  // The deadline's is the grid alone, which fits above the keyboard.
  const madeRoom = useRef(false);
  const beforeCalendar = useRef(initial);
  const roomFrom = useRef<HTMLElement | null>(null);
  const startCalendar = () => {
    // What the calendar's X goes back to.
    beforeCalendar.current = latest.current;
  };
  const makeRoom = () => {
    startCalendar();
    if (!touch) return;
    madeRoom.current = true;
    // The field the keyboard is up for, unless it's inside the picker being replaced.
    const focused = document.activeElement as HTMLElement | null;
    roomFrom.current =
      focused?.closest(".on-keyboard") && !focused.closest(".popover") ? focused : title.current;
    // The card sets off with the keyboard, not when Swift reports it, and the
    // keyboard a frame after: iOS moves it at once, the card's ride reaches the
    // screen a frame or two after the page starts it.
    if (focused?.closest("input, textarea") && lowerKeyboardNow())
      requestAnimationFrame(() => focused.blur());
    else focused?.blur();
  };
  const setPop = (p: Pop) => {
    setOpen(p);
    if (madeRoom.current) {
      madeRoom.current = false;
      (roomFrom.current ?? title.current)?.focus();
    } else if (touch && document.activeElement?.closest(".popover")) {
      // The picker's own field is going away; the keyboard stays for the title.
      title.current?.focus();
    }
    if (p === "deadline") startCalendar();
  };
  /** Going to the title or description closes the picker; that field keeps the keyboard. */
  const closeForField = () => {
    if (open === null) return;
    madeRoom.current = false;
    setPop(null);
  };
  // Pointer up, since iOS sends no click for a tap in the field that has the cursor.
  const tapField = (e: PointerEvent<HTMLElement>) => {
    if ((e.target as HTMLElement).closest(".title-input, .desc")) closeForField();
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
  const view = editorView(values, quickAdd, keepLabel, existing, projects, sections, labels, today);
  const pick = (p: EditorPick) => setValues((v) => applyPick(v, p, quickAdd, projects, sections, today));
  const typedRepeat = {
    read: (text: string) => readRepeat(text, view.shown.date, today)?.text ?? null,
    onSubmit: (text: string) => pick({ type: "repeatText", text }),
  };

  const submit = () => {
    const v = latest.current;
    if (
      submitted.current ||
      !editorView(v, quickAdd, keepLabel, existing, projects, sections, labels, today).can_submit
    )
      return;
    if (keepOpen) {
      latest.current = initial;
      setValues(initial);
      setPop(null);
      // The next task starts with its title, wherever this one ended.
      title.current?.focus();
    } else {
      submitted.current = true;
    }
    onSubmit(v);
  };

  useRevealFilled(pillRow.ref, typedPills(view));

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
        date={view.shown.date}
        time={view.shown.time}
        text={view.date_chip[0]}
        bucket={view.date_chip[1]}
        rule={view.repeat_rule}
        repeat={{
          items: repeatItems,
          selected: view.repeat_selected,
          text: repeatItems.find((i) => i.value === view.repeat_selected)?.label ?? view.repeat_chip_text,
          onSelect: (value) => pick({ type: "repeat", value }),
          typed: typedRepeat,
        }}
        today={today}
        open={open}
        setOpen={setPop}
        onChange={(date, time) => pick({ type: "date", date, time })}
        onCalendar={makeRoom}
        actions={calendarActions}
      />
      <DeadlinePicker
        deadline={view.shown.deadline}
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
        iconClass={flagClass(view.shown.priority)}
        text={view.priority_text}
        items={PRIORITY_ITEMS}
        selected={view.shown.priority}
        open={open}
        setOpen={setPop}
        chipClass={
          ["1", "2", "3"].includes(view.shown.priority)
            ? `pill-p${view.shown.priority} pill-tinted pill-set`
            : ""
        }
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
        chipClass={locationChosen(view.locations, view.location) ? "pill-set pill-project" : ""}
        onSelect={(value) => pick(locationPick(value))}
      />
      <LabelPicker
        labels={labels}
        selected={view.shown.labels}
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
        selected={view.repeat_selected}
        open={open}
        setOpen={setPop}
        chipClass={view.repeat_rule !== null ? "pill-set pill-repeat" : ""}
        onSelect={(value) => pick({ type: "repeat", value })}
        typed={typedRepeat}
      />
      {view.streak_offered ? (
        <StreakPicker
          text={streakPillText(view)}
          on={view.shown.streak}
          goal={view.shown.streak_goal}
          open={open}
          setOpen={setPop}
          onPick={(on, goal) => pick({ type: "streak", on, goal })}
          onEnter={() => {
            setPop(null);
            title.current?.focus();
          }}
        />
      ) : null}
    </div>
  );

  const clickAway = (
    // biome-ignore lint/a11y/useKeyWithClickEvents: Escape closes the popover from the form; this is the click-away.
    // biome-ignore lint/a11y/noStaticElementInteractions: a click-away layer, as in the Dioxus editor.
    <div className="pop-backdrop" onClick={() => setPop(null)} onWheel={() => setPop(null)} />
  );

  const form = (
    <form
      className={cx(
        "editor",
        touch && "on-keyboard",
        touch && !keyboard.up && "keyboard-down",
        touch && open !== null && "picking",
      )}
      style={
        touch
          ? ({
              bottom: keyboard.inset + CARD_GAP,
              "--card-room": `${keyboard.room}px`,
            } as CSSProperties)
          : undefined
      }
      // Pressing anything but a text field keeps the title focused, so the keyboard stays up.
      onMouseDown={touch ? keepFocus : undefined}
      onPointerUp={tapField}
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
      {/* The part that scrolls once the card fills the room above the keyboard. */}
      <div className="editor-text">
        <HighlightedTitle
          segments={view.segments}
          labels={labels}
          value={values.content}
          placeholder={quickAdd ? "Task name — try “Call mom tomorrow 5pm p1 #Home @phone”" : "Task name"}
          onInput={(text) => {
            closeForField();
            setValues((v) => withTitle(v, text));
          }}
          onEnter={submit}
          onUnhighlight={(caret) => {
            closeForField();
            setValues((v) => (quickAdd ? (unhighlightAt(v, projects, sections, today, caret) ?? v) : v));
          }}
          inputRef={title}
        />
        <div className="desc-wrap">
          <div className="desc-mirror" aria-hidden="true">
            {`${values.description}\u200b`}
          </div>
          <textarea
            className="desc"
            rows={1}
            placeholder="Description"
            value={values.description}
            onChange={(e) => {
              const description = e.target.value;
              closeForField();
              setValues((v) => ({ ...v, description }));
            }}
            onKeyDown={(e) => {
              // Enter is a new line here; the modifier is the keyboard's way to save.
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                submit();
              }
            }}
          />
        </div>
      </div>
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
  // The dim layer sits beside the card: inside it, it would dim the card's own
  // color. One element throughout, so a picker closing doesn't restart its fade.
  return createPortal(
    <>
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: Escape acts from the form; this is the tap outside. */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: a dimmed tap-away layer behind the card. */}
      <div
        className={open !== null ? "editor-scrim pop-backdrop" : "editor-scrim"}
        onClick={open !== null ? () => setPop(null) : onCancel}
        onWheel={open !== null ? () => setPop(null) : undefined}
        onMouseDown={keepFocus}
      />
      {form}
    </>,
    document.body,
  );
}
