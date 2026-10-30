// The Today view, read-only: the markup and class names of the Dioxus
// MainPane / CardList / TaskRow (crates/ui/src/main.rs, cards.rs,
// task_row.rs), so app.css applies unchanged and Demo.swift and
// scripts/sim-checks find what they query (.task-row, .task-body,
// .task-content, .task-badges, .task-count, .task-project).
//
// Every decision about the tasks comes from keel-core: which tasks are due
// (today_view), how they group into cards and count sub-tasks (cards), and
// the chip text (dueDateOf / dueLabel / dueTime / timeLabel).
import { useEffect, useState } from "react";
import type { Card, CardRow, Priority, Progress, Project, Section, Task } from "../bindings";
import { daySubtitle, localToday } from "../clock";
import { cards, dueDateOf, dueLabel, dueTime, timeLabel } from "../core";
import {
  CalendarSmallIcon,
  ChevronRightIcon,
  DoneMark,
  FlagFilledIcon,
  MenuIcon,
  RepeatSmallIcon,
  TagSmallIcon,
} from "../shell/icons";
import { loadToday, type TodayData } from "./today-data";

/** Where a task lives, "Project" or "Project / Section" (Dioxus `location_names`). */
function locationOf(task: Task, projects: Project[], sections: Section[]): string {
  const project = projects.find((p) => p.id === task.project_id)?.name ?? "";
  const section = task.section_id === null ? undefined : sections.find((s) => s.id === task.section_id);
  return section ? `${project} / ${section.name}` : project;
}

/** The due chip's text, "Today" or "Friday 5 PM"; null without a due. */
function dueText(task: Task, today: string): string | null {
  const date = dueDateOf(task);
  if (date === null) return null;
  const label = dueLabel(date, today);
  const time = dueTime(task);
  return time === null ? label : `${label} ${timeLabel(time)}`;
}

const CHECK_CLASS: Record<Priority, string> = {
  P1: "check p1",
  P2: "check p2",
  P3: "check p3",
  None: "check",
};
const FLAG_CLASS: Record<Priority, string | null> = {
  P1: "card-flag p1",
  P2: "card-flag p2",
  P3: "card-flag p3",
  None: null,
};

// "x/y" (Rust `Progress::short`). The Dioxus row adds a green check when
// all are done; that test (`Progress::all_done`) is a keel-core rule with no
// wasm export yet, so it waits for one rather than being copied here.
function SubtaskCount({ progress }: { progress: Progress }) {
  return <span className="task-count">{`${progress.done}/${progress.total}`}</span>;
}

function TaskRow({ row, today, location }: { row: CardRow; today: string; location: string }) {
  const { task, progress } = row;
  const due = dueText(task, today);
  const showMeta = due !== null || task.labels.length > 0;
  return (
    <div className="task" data-menu-id={task.id}>
      <div className="task-row">
        <button type="button" className={CHECK_CLASS[task.priority]} title="Complete task">
          <DoneMark />
        </button>
        <div className="task-body" title="Open task">
          <div className="task-title">
            <div className="task-content">{task.content}</div>
          </div>
          {task.description ? <div className="task-desc">{task.description}</div> : null}
          {showMeta ? (
            <div className="task-meta">
              {due !== null ? (
                // Dioxus colors the chip by keel-core's `due_bucket`
                // (overdue, today, …), which has no wasm export yet.
                <span className="due">
                  <CalendarSmallIcon />
                  {due}
                  {task.due?.is_recurring ? <RepeatSmallIcon /> : null}
                </span>
              ) : null}
              {task.labels.map((name) => (
                <span key={name} className="label-tag">
                  <TagSmallIcon />
                  {name}
                </span>
              ))}
            </div>
          ) : null}
        </div>
        <div className="task-side">
          <div className="task-badges">
            {progress ? <SubtaskCount progress={progress} /> : null}
            {task.streak_enabled ? (
              <span className="streak" title="Current streak">{`🔥 ${task.streak_current}`}</span>
            ) : null}
          </div>
          <span className="task-project">{location}</span>
        </div>
      </div>
    </div>
  );
}

/** A card's header when its task doesn't match the view: context only, no circle. */
function CardHead({ row, today }: { row: CardRow; today: string }) {
  const { task, progress } = row;
  const flag = FLAG_CLASS[task.priority];
  const due = dueText(task, today);
  return (
    <div className="card-head" title="Open task">
      {flag ? (
        <span className={flag}>
          <FlagFilledIcon />
        </span>
      ) : null}
      <span className="pname">{task.content}</span>
      <span className="card-head-side">
        {due !== null ? <span className="due">{due}</span> : null}
        {progress ? <SubtaskCount progress={progress} /> : null}
        <span className="card-chev">
          <ChevronRightIcon />
        </span>
      </span>
    </div>
  );
}

function CardList({ list, data, today }: { list: Card[]; data: TodayData; today: string }) {
  const where = (t: Task) => locationOf(t, data.projects, data.sections);
  return list.map((card) => (
    <div key={card.head.task.id} className="task-card">
      {card.head_matches ? (
        <TaskRow row={card.head} today={today} location={where(card.head.task)} />
      ) : (
        <CardHead row={card.head} today={today} />
      )}
      {card.rows.map((row) => (
        <TaskRow key={row.task.id} row={row} today={today} location={where(row.task)} />
      ))}
    </div>
  ));
}

export function Today({ load = loadToday }: { load?: () => Promise<TodayData> }) {
  const [data, setData] = useState<TodayData | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    load().then(
      (d) => {
        if (live) setData(d);
      },
      (e: unknown) => {
        if (live) setError(String(e));
      },
    );
    return () => {
      live = false;
    };
  }, [load]);

  if (error !== null) return <div className="error">Error: {error}</div>;
  const today = localToday();
  // Grouped, as the Dioxus Today (`view_mode`): matched sub-tasks sit in
  // their top-level parent's card.
  const list = data ? cards(data.matched, data.all, "Grouped") : [];
  return (
    <div className="content today">
      <div className="view-head">
        <button type="button" className="icon-btn menu-btn" title="Open menu" aria-label="Open menu">
          <MenuIcon />
        </button>
        <h1 className="view-title">Today</h1>
      </div>
      <p className="view-sub">{data ? daySubtitle() : "Loading…"}</p>
      {data && list.length === 0 ? (
        <div className="empty">
          <div className="big">You're all done for today!</div>
          <div>Enjoy the rest of your day.</div>
        </div>
      ) : null}
      {data ? <CardList list={list} data={data} today={today} /> : null}
    </div>
  );
}
