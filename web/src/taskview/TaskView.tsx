// The task view: header, title line, fields card and sub-task card for one
// level of the view stack (crates/ui/src/task_view/view.rs `TaskView`, without
// the native glass branches). Every change saves right away through
// `saveEdit`, the same path as the editor; saves are fire-and-forget, so the
// view moves at once and the reload that follows catches up.
import { useEffect, useMemo, useRef, useState } from "react";
import { flushSync } from "react-dom";
import type { Loaded } from "../app/load";
import type { EditorPick, EditorValues, Id, TaskDetail } from "../bindings";
import { localToday } from "../clock";
import {
  applyPick,
  editorView,
  fieldLayout,
  locationText,
  reminderDraftOf,
  repeatText,
  toggleLabel,
  valuesFromTask,
} from "../core";
import type { Pop } from "../editor/pop";
import { saveEdit } from "../editor/save";
import { slideClose } from "../gestures/sheet";
import { completeTask, deleteTask, uncompleteTask } from "../ipc/actions";
import { CHECK_CLASS } from "../lists/chips";
import { StreakPanel } from "../lists/StreakPanel";
import { CheckIcon, ChevronLeftIcon, MoreIcon, TextLinesIcon, XIcon } from "../shell/icons";
import { commitDrafts, takesStoredText } from "./drafts";
import { FieldControl } from "./FieldControl";
import { GrowingText } from "./GrowingText";
import { MoreMenu } from "./MoreMenu";
import { useTaskNav } from "./nav-context";
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
  const setTitle = (s: string) => {
    titleRef.current = s;
    setTitleState(s);
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
  const [streak, setStreak] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const [open, setOpen] = useState<Pop>(null);

  const save = (pair: { before: EditorValues; after: EditorValues }) =>
    saveEdit(id, pair.before, pair.after, rule, localToday());
  // The title / description edit to save, if any (`base` moves ahead). An
  // emptied title goes back to the saved one.
  const commit = () => {
    const { title: shown, save: pair } = commitDrafts(baseRef.current, titleRef.current, descRef.current);
    if (shown !== titleRef.current) setTitle(shown);
    if (pair !== null) setBase(pair.after);
    return pair;
  };
  const saveDrafts = () => {
    const pair = commit();
    // Not tied to the view: it may close before the save lands.
    if (pair !== null) void settle(save(pair)).then(onChanged);
  };
  // Moves at once (a second fast tap on Back acts on the view below, not
  // again on this one); the pending edit still saves.
  const navigate = (to: NavTo) => {
    saveDrafts();
    if (to === "back") nav.back();
    else if (to === "close") nav.close();
    else nav.push(to.push);
  };
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

  // A picker's change, saved now.
  const saveValues = (after: EditorValues) => {
    const before = baseRef.current;
    if (sameValues(before, after)) return;
    setBase(after);
    void settle(save({ before, after })).then(onChanged);
  };
  const pick = (p: EditorPick) => saveValues(applyPick(baseRef.current, p, false, projects, sections, today));
  const toggle = (name: string) =>
    saveValues(toggleLabel(baseRef.current, name, false, projects, sections, today));

  const completeHere = () => {
    if (!nav.holdCircles()) return;
    const pair = commit();
    const checked = task.checked;
    void (async () => {
      if (pair !== null) await settle(save(pair));
      if (checked) await settle(uncompleteTask(id));
      // Back to the parent, or closed at the top.
      else if (await settle(completeTask(id))) nav.completedHere();
      onChanged();
      await nav.releaseCircles();
    })();
  };

  const v: EditorValues = { ...base, description: desc };
  const view = editorView(v, false, keepLabel, rule, projects, sections, today);
  const { fields, missing } = fieldsShown(fieldLayout(v, loaded.inbox.id), descOpen);
  const crumb = detail.parent?.content ?? locationText(task.project_id, null, projects, sections);
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
      {active ? (
        <div className="tv-head">
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
          {depth > 1 ? (
            <button type="button" className="tv-back icon-btn" title="Back" onClick={() => navigate("back")}>
              <ChevronLeftIcon />
              <span>Back</span>
            </button>
          ) : null}
          <span className="tv-crumb">{crumb}</span>
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
                onStreak={() => {
                  setMoreOpen(false);
                  setStreak((s) => !s);
                }}
                onDelete={() => {
                  setMoreOpen(false);
                  // The reload prunes the stack: this view and any above it close.
                  void settle(deleteTask(id)).then(onChanged);
                }}
              />
            ) : null}
          </div>
        </div>
      ) : (
        <div className="tv-head-space" />
      )}
      <div className="tv-body">
        <div className="tv-scroll">
          <div className={task.checked ? "tv-titleline done" : "tv-titleline"}>
            <button
              type="button"
              className={task.checked ? "check done" : CHECK_CLASS[task.priority]}
              title={task.checked ? "Mark as not done" : "Complete task"}
              onClick={completeHere}
            >
              <CheckIcon />
            </button>
            <GrowingText
              className="tv-title"
              value={title}
              placeholder="Task name"
              onInput={setTitle}
              onCommit={saveDrafts}
            />
          </div>
          <div className="tv-card tv-fields">
            {fields.map((f) => (
              <div key={f} className="tv-row">
                {f === "Description" ? (
                  <>
                    <span className="tv-row-icon">
                      <TextLinesIcon />
                    </span>
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
                  </>
                ) : (
                  <FieldControl field={f} missing={null} {...fieldProps} />
                )}
              </div>
            ))}
            {missing.length > 0 ? (
              <div className="tv-pills">
                {missing.map((m) => (
                  <span key={m.label} className="tv-pill-slot">
                    <FieldControl field={null} missing={m} {...fieldProps} />
                  </span>
                ))}
              </div>
            ) : null}
          </div>
          {streak && recurring ? (
            <div className="tv-card">
              <StreakPanel task={task} onChanged={onChanged} />
            </div>
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
