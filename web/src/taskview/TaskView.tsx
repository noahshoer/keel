// The task view: header, title line, fields card and sub-task card for one
// level of the view stack (crates/ui/src/task_view/view.rs `TaskView`, without
// the native glass branches). Every change saves right away through
// `saveEdit`, the same path as the editor; saves are fire-and-forget, so the
// view moves at once and the reload that follows catches up.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { flushSync } from "react-dom";
import type { Loaded } from "../app/load";
import type { EditorPick, EditorValues, Id, Recurrence, TaskDetail } from "../bindings";
import { localToday } from "../clock";
import {
  applyPick,
  applyTitle,
  editorView,
  fieldLayout,
  movesOccurrence,
  reminderDraftOf,
  repeatText,
  streakText,
  toggleLabel,
  unhighlightAt,
  valuesFromTask,
  withStoredPlain,
  withTitle,
} from "../core";
import { isTouch } from "../editor/locale";
import { usePillFade } from "../editor/pillFade";
import type { Pop } from "../editor/pop";
import { saveEdit } from "../editor/save";
import { slideClose } from "../gestures/sheet";
import { completeTask, deleteTask, uncompleteTask } from "../ipc/actions";
import { CHECK_CLASS } from "../lists/chips";
import { openAndFocus } from "../shell/focus";
import { haptic } from "../shell/haptics";
import {
  CheckIcon,
  ChevronLeftIcon,
  FireIcon,
  MoreIcon,
  TargetSmallIcon,
  TrophySmallIcon,
  XIcon,
} from "../shell/icons";
import { commitDrafts, takesStoredText } from "./drafts";
import { FieldControl } from "./FieldControl";
import { GrowingText } from "./GrowingText";
import { MoreMenu } from "./MoreMenu";
import { MovePrompt } from "./MovePrompt";
import { useTaskNav } from "./nav-context";
import { StreakOnKeyboard, StreakPopover } from "./StreakPopover";
import { SubtaskCard } from "./SubtaskCard";
import { fieldsShown, sameValues, settle } from "./view-rules";

/** Where the view goes after saving a pending title or description. */
type NavTo = { push: Id } | "back" | "close";

export function TaskView({
  detail,
  depth,
  active,
  loaded,
  onChanged,
}: {
  detail: TaskDetail;
  /** How many views are stacked (Back shows above one). */
  depth: number;
  /**
   * False for the view under a sub-task's, kept mounted and out of sight for
   * the edge swipe: no header (a spacer of its height), and it leaves the
   * shared flags (add sub-task, close) to the top one.
   */
  active: boolean;
  loaded: Loaded;
  /** Reload after a change. */
  onChanged: () => void;
}) {
  const nav = useTaskNav();
  const { projects, sections, labels } = loaded;
  const today = localToday();
  const task = detail.task;
  const id = task.id;
  const rule = task.due?.recurrence ?? null;
  const recurring = task.due?.is_recurring ?? false;

  // The task as the editor sees it, with its reminders.
  const stored = useMemo<EditorValues>(
    () => ({
      ...valuesFromTask(task),
      reminders: loaded.reminders.filter((r) => r.task_id === task.id).map(reminderDraftOf),
    }),
    [task, loaded.reminders],
  );
  const keepLabel = stored.repeat === "keep" ? (rule !== null ? repeatText(rule) : "") : null;

  // `base` is what was last saved (ahead of the reload that confirms it);
  // `title` and `desc` are the drafts typed in place. Refs mirror them, so a
  // second call in the same tick sees the first one's change.
  const [base, setBaseState] = useState(stored);
  const baseRef = useRef(stored);
  const setBase = (v: EditorValues) => {
    baseRef.current = v;
    setBaseState(v);
  };
  const seen = useRef(stored);
  const [title, setTitleState] = useState(stored.content);
  const titleRef = useRef(stored.content);
  // The phrases in the title that stay text: all of the stored title's, until
  // they're retyped, and any backspaced to plain (so they don't apply).
  const plainOf = (content: string) =>
    withStoredPlain({ ...stored, content, plain: [] }, projects, sections, today).plain;
  const plainRef = useRef(plainOf(stored.content));
  const [plain, setPlainState] = useState(plainRef.current);
  const setPlain = (p: Array<[number, number]>) => {
    plainRef.current = p;
    setPlainState(p);
  };
  // The title replaced by the stored one or a save.
  const setTitle = (s: string) => {
    titleRef.current = s;
    setTitleState(s);
    setPlain(plainOf(s));
  };
  // The title typed: the kept-plain phrases follow the text.
  const typeTitle = (s: string) => {
    const typed = withTitle({ ...baseRef.current, content: titleRef.current, plain: plainRef.current }, s);
    titleRef.current = s;
    setTitleState(s);
    setPlain(typed.plain);
  };
  const [desc, setDescState] = useState(stored.description);
  const descRef = useRef(stored.description);
  const setDesc = (s: string) => {
    descRef.current = s;
    setDescState(s);
  };
  // The stored task changed (a save landed, or an edit elsewhere): take it,
  // and reset only the drafts whose stored text changed. Our own save coming
  // back leaves a draft typed since alone.
  // biome-ignore lint/correctness/useExhaustiveDependencies: runs when the stored values change; the setters write refs.
  useEffect(() => {
    const old = seen.current;
    if (sameValues(stored, old)) return;
    const saved = baseRef.current;
    if (takesStoredText(stored.content, old.content, saved.content)) setTitle(stored.content);
    if (takesStoredText(stored.description, old.description, saved.description)) setDesc(stored.description);
    setBase(stored);
    seen.current = stored;
  }, [stored]);

  const [descOpen, setDescOpen] = useState(false);
  const [adding, setAdding] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const [open, setOpen] = useState<Pop>(null);

  type Pair = { before: EditorValues; after: EditorValues; rule: Recurrence | null };
  // A move of one occurrence resets a streak going, so it waits for an answer.
  type PendingMove = { move: () => void; cancel: () => void };
  const [pendingMove, setPendingMoveState] = useState<PendingMove | null>(null);
  const pendingRef = useRef<PendingMove | null>(null);
  const setPendingMove = (p: PendingMove | null) => {
    pendingRef.current = p;
    setPendingMoveState(p);
  };
  const asksFirst = (before: EditorValues, after: EditorValues) =>
    task.streak_enabled && task.streak_current > 0 && movesOccurrence(before, after, rule);
  // Reads the ref, so it keeps its identity (the prompt's outside-click close).
  const cancelMove = useCallback(() => {
    const p = pendingRef.current;
    pendingRef.current = null;
    setPendingMoveState(null);
    p?.cancel();
  }, []);
  const save = (pair: Pair) => saveEdit(id, pair.before, pair.after, pair.rule, localToday());
  // The title / description edit to save, if any (`base` moves ahead). An
  // emptied title goes back to the saved one. A retyped title applies what it
  // says (a date, a priority, ...) and loses those words; one with nothing
  // else in it goes back too.
  // What the drafts would save (`pair`) and the title they leave, without
  // applying either; `take` applies it.
  type Draft = { pair: Pair | null; title: string };
  const readDrafts = (): Draft => {
    const before = baseRef.current;
    const typed = titleRef.current.trim();
    const retyped = typed !== "" && typed !== before.content;
    const applied = retyped
      ? applyTitle(
          { ...before, content: titleRef.current, description: descRef.current, plain: plainRef.current },
          rule,
          projects,
          sections,
          today,
        )
      : null;
    if (applied !== null) {
      const after = { ...applied.values, description: descRef.current };
      return {
        pair: sameValues(before, after) ? null : { before, after, rule: applied.rule },
        title: after.content,
      };
    }
    const drafts = commitDrafts(before, retyped ? before.content : titleRef.current, descRef.current);
    return { pair: drafts.save === null ? null : { ...drafts.save, rule }, title: drafts.title };
  };
  const take = (d: Draft) => {
    if (d.title !== titleRef.current) setTitle(d.title);
    if (d.pair !== null) {
      setBase(d.pair.after);
      // Not tied to the view: it may close before the save lands.
      void settle(save(d.pair)).then(onChanged);
    }
  };
  // Saves the drafts, then `then`. A typed date that would reset a streak
  // asks first; Cancel keeps the description but not the retyped title.
  const saveDrafts = (then?: () => void) => {
    const d = readDrafts();
    if (d.pair !== null && asksFirst(d.pair.before, d.pair.after)) {
      setOpen(null);
      setPendingMove({
        move: () => {
          take(d);
          then?.();
        },
        cancel: () => {
          const before = baseRef.current;
          const drafts = commitDrafts(before, before.content, descRef.current);
          take({ pair: drafts.save === null ? null : { ...drafts.save, rule }, title: before.content });
          then?.();
        },
      });
      return;
    }
    take(d);
    then?.();
  };
  // Moves at once (a second fast tap on Back acts on the view below, not
  // again on this one); the pending edit still saves. Only a move that would
  // reset a streak holds it, until it's answered.
  const navigate = (to: NavTo) =>
    saveDrafts(() => {
      if (to === "back") nav.back();
      else if (to === "close") nav.close();
      else nav.push(to.push);
    });
  // A list row's "Add sub-task" opens the view with the editor showing.
  useEffect(() => {
    if (active && nav.addSub) {
      nav.clearAddSub();
      setAdding(true);
    }
  }, [active, nav]);
  // A tap on the scrim or a drag that pulled the sheet down.
  const navigateRef = useRef(navigate);
  navigateRef.current = navigate;
  useEffect(() => {
    if (active && nav.closeAsked) {
      nav.clearCloseAsked();
      navigateRef.current("close");
    }
  }, [active, nav]);

  // A picker's change, saved now; a move that would reset a streak asks first.
  const fieldsCard = useRef<HTMLDivElement>(null);
  const commitValues = (after: EditorValues) => {
    const before = baseRef.current;
    if (sameValues(before, after)) return;
    setBase(after);
    void settle(save({ before, after, rule })).then(onChanged);
  };
  const saveValues = (after: EditorValues) => {
    if (asksFirst(baseRef.current, after)) {
      setOpen(null);
      setPendingMove({ move: () => commitValues(after), cancel: () => {} });
    } else {
      commitValues(after);
    }
  };
  // Today at pick time, as Dioxus reads it: a view left open past midnight.
  const pick = (p: EditorPick) =>
    saveValues(applyPick(baseRef.current, p, false, projects, sections, localToday()));
  const toggle = (name: string) =>
    saveValues(toggleLabel(baseRef.current, name, false, projects, sections, localToday()));

  // On touch a tracked streak opens on the keyboard, which only a focus inside
  // the tap raises. Otherwise it hangs from what opened it.
  const goalField = useRef<HTMLInputElement>(null);
  const openStreak = (from: "streak-row" | "streak-more") => {
    const onKeyboard = isTouch() && task.streak_enabled;
    openAndFocus(() => {
      setMoreOpen(false);
      setOpen(onKeyboard ? "streak-keyboard" : open === from ? null : from);
    }, goalField);
  };

  const completeHere = () => {
    if (!nav.holdCircles()) return;
    // Completing saves the drafts first, without asking: the occurrence is done either way.
    const { pair, title } = readDrafts();
    if (title !== titleRef.current) setTitle(title);
    if (pair !== null) setBase(pair.after);
    const done = detail.done;
    if (!done) haptic("complete");
    void (async () => {
      if (pair !== null) await settle(save(pair));
      if (done) await settle(uncompleteTask(id));
      // Back to the parent, or closed at the top.
      else if (await settle(completeTask(id))) nav.completedHere();
      onChanged();
      await nav.releaseCircles();
    })();
  };

  // The fields show what the title being typed says (and the stored values
  // again once a phrase is backspaced to plain).
  const draft: EditorValues = { ...base, content: title, description: desc, plain };
  const view = editorView(draft, true, keepLabel, rule, projects, sections, today);
  const v: EditorValues = { ...view.shown, content: base.content, description: desc };
  const { fields, missing } = fieldsShown(fieldLayout(v, loaded.inbox.id), descOpen);
  const rows = fields.filter((f) => f !== "Description");
  const pillRow = usePillFade(missing.length);
  const fieldProps = {
    values: v,
    view,
    labels,
    today,
    open,
    setOpen,
    onPick: pick,
    onToggle: toggle,
    onLabelsChanged: onChanged,
    onOpenDescription: () => flushSync(() => setDescOpen(true)),
  };

  return (
    <div
      className={active ? "task-view" : "task-view tv-under"}
      data-view-id={task.id}
      aria-hidden={active ? "false" : "true"}
    >
      {open !== null || moreOpen ? (
        // biome-ignore lint/a11y/useKeyWithClickEvents: a transparent click-away layer, as in the Dioxus view.
        // biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI.
        <div
          className="pop-backdrop"
          onClick={() => {
            setOpen(null);
            setMoreOpen(false);
          }}
        />
      ) : null}
      {open === "streak-keyboard" ? (
        <StreakOnKeyboard task={task} onChanged={onChanged} goal={goalField} />
      ) : null}
      {active ? (
        <div className="tv-head">
          {depth > 1 ? (
            <button
              type="button"
              className="tv-back tv-round"
              title="Back"
              aria-label="Back"
              onClick={() => navigate("back")}
            >
              <ChevronLeftIcon />
            </button>
          ) : (
            <button
              type="button"
              className="tv-close tv-round"
              title="Close"
              aria-label="Close"
              // On the phone layout the sheet slides down first.
              onClick={() => {
                if (!slideClose(document, window)) navigate("close");
              }}
            >
              <XIcon />
            </button>
          )}
          <div className="chip-wrap tv-head-actions">
            <button
              type="button"
              className="tv-more tv-round"
              title="More"
              aria-label="More"
              onClick={() => setMoreOpen((o) => !o)}
            >
              <MoreIcon />
            </button>
            {moreOpen ? (
              <MoreMenu
                recurring={recurring}
                onStreak={() => openStreak("streak-more")}
                onDelete={() => {
                  setMoreOpen(false);
                  // The reload prunes the stack: this view and any above it close.
                  void settle(deleteTask(id)).then(onChanged);
                }}
              />
            ) : null}
            {open === "streak-more" ? <StreakPopover task={task} onChanged={onChanged} /> : null}
          </div>
        </div>
      ) : (
        <div className="tv-head-space" />
      )}
      <div className="tv-body">
        <div className="tv-scroll">
          <div className={detail.done ? "tv-titleline done" : "tv-titleline"}>
            <button
              type="button"
              className={detail.done ? "check done" : CHECK_CLASS[task.priority]}
              title={detail.done ? "Mark as not done" : "Complete task"}
              onClick={completeHere}
            >
              <CheckIcon />
            </button>
            <GrowingText
              className="tv-title"
              value={title}
              placeholder="Task name"
              segments={view.segments}
              labels={labels}
              onUnhighlight={(caret) => {
                const next = unhighlightAt(draft, projects, sections, today, caret);
                if (next !== null) setPlain(next.plain);
              }}
              onInput={typeTitle}
              onCommit={() => saveDrafts()}
            />
          </div>
          {fields.includes("Description") ? (
            <div className="tv-desc-line">
              <GrowingText
                className="tv-desc"
                value={desc}
                placeholder="Description"
                multiline
                autoFocus={descOpen && baseRef.current.description.trim() === ""}
                onInput={setDesc}
                onCommit={() => {
                  saveDrafts();
                  setDescOpen(false);
                }}
              />
            </div>
          ) : null}
          {rows.length > 0 || task.streak_enabled ? (
            <div ref={fieldsCard} className="tv-card tv-fields">
              {task.streak_enabled ? (
                <div className="tv-row">
                  <div className="chip-wrap">
                    <button
                      type="button"
                      className="tv-field-btn tv-streak"
                      onClick={() => openStreak("streak-row")}
                    >
                      <FireIcon />
                      {streakText(task)}
                      <span className="tv-streak-best">
                        <span className="tv-streak-stat" title="Best">
                          <TrophySmallIcon />
                          {task.streak_best}
                        </span>
                        {task.streak_goal === null ? null : (
                          <span className="tv-streak-stat" title="Goal">
                            <TargetSmallIcon />
                            {task.streak_goal}
                          </span>
                        )}
                      </span>
                    </button>
                    {open === "streak-row" ? <StreakPopover task={task} onChanged={onChanged} /> : null}
                  </div>
                </div>
              ) : null}
              {rows.flatMap((f) => {
                const row = (
                  <div key={f} className="tv-row">
                    <FieldControl field={f} missing={null} {...fieldProps} />
                  </div>
                );
                return f === "Date" && v.repeat !== "none"
                  ? [
                      row,
                      <div key="repeat" className="tv-row">
                        <FieldControl field="Repeat" missing={null} {...fieldProps} />
                      </div>,
                    ]
                  : [row];
              })}
            </div>
          ) : null}
          {missing.length > 0 ? (
            <div className={`tv-pills-wrap ${pillRow.fadeClass}`}>
              <div ref={pillRow.ref} className="tv-pills" onScroll={pillRow.onScroll}>
                {missing.map((m) => (
                  <span key={m.label} className="tv-pill-slot">
                    <FieldControl field={null} missing={m} {...fieldProps} />
                  </span>
                ))}
              </div>
            </div>
          ) : null}
          {pendingMove !== null ? (
            <MovePrompt
              anchor={fieldsCard}
              streak={task.streak_current}
              onCancel={cancelMove}
              onMove={() => {
                setPendingMove(null);
                pendingMove.move();
              }}
            />
          ) : null}
          <SubtaskCard
            detail={detail}
            loaded={loaded}
            adding={adding}
            setAdding={setAdding}
            onOpen={(sub) => navigate({ push: sub })}
            onChanged={onChanged}
          />
        </div>
      </div>
    </div>
  );
}
