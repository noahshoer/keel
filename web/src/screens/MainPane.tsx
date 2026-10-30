// The main pane for every view: the markup and branches of the Dioxus
// `MainPane` (crates/ui/src/main.rs). Dated views split into keel-core's
// date groups, filters into their comma groups, projects into sections.
// The "+ Add task" rows come with the editor (Plan 3).

import type { Loaded } from "../app/load";
import {
  canShowCompleted,
  emptyText,
  type Motion,
  showsProject,
  viewMode,
  viewSubtitle,
  viewTitle,
} from "../app/view";
import type { Id, Section, Task } from "../bindings";
import { localToday } from "../clock";
import { cards, dateGroups } from "../core";
import { CardList } from "../lists/CardList";
import { CompletedRow } from "../lists/CompletedRow";
import { CheckCircleIcon, MenuIcon } from "../shell/icons";
import { AddSection, SectionBlock } from "./SectionBlock";

/** "Project" or "Project / Section" for each task the pane lists. */
function locationNames(loaded: Loaded): Map<Id, string> {
  const projects = new Map(loaded.projects.map((p) => [p.id, p.name]));
  const sections = new Map(loaded.sections.map((s) => [s.id, s.name]));
  const out = new Map<Id, string>();
  for (const t of [...loaded.tasks, ...loaded.completed]) {
    const project = projects.get(t.project_id) ?? "";
    const section = t.section_id === null ? undefined : sections.get(t.section_id);
    out.set(t.id, section === undefined ? project : `${project} / ${section}`);
  }
  return out;
}

function contentClass(motion: Motion, swap: number): string {
  if (motion === "pop") return "content view-pop";
  if (motion === "push") return "content";
  return swap % 2 === 0 ? "content view-in-a" : "content view-in-b";
}

export function MainPane({
  loaded,
  showCompleted,
  onToggleCompleted,
  onOpenDrawer,
  motion,
  swap,
  onChanged,
  children,
}: {
  loaded: Loaded;
  showCompleted: boolean;
  onToggleCompleted: () => void;
  onOpenDrawer: () => void;
  motion: Motion;
  /** Bumped when a different view lands; alternates the fade-in. */
  swap: number;
  onChanged: () => void;
  /** The Search pane, in the Search view. */
  children?: React.ReactNode;
}) {
  const { view } = loaded;
  const today = localToday();
  const mode = viewMode(view);
  const toggle = canShowCompleted(view);
  const subtitle = viewSubtitle(view);
  const locations = showsProject(view) ? locationNames(loaded) : null;
  const search = view.kind === "search";
  const projectId = view.kind === "project" ? view.id : view.kind === "inbox" ? loaded.inbox.id : null;
  const projectSections: Section[] =
    projectId === null ? [] : loaded.sections.filter((s) => s.project_id === projectId);
  const inSection = (t: Task, s: Section) => t.section_id === s.id;
  const unsectioned = loaded.tasks.filter((t) => !projectSections.some((s) => inSection(t, s)));
  const empty = loaded.tasks.length === 0;
  const [big, small] = emptyText(view);
  const list = (tasks: Task[]) => (
    <CardList
      cards={cards(tasks, loaded.all, mode)}
      today={today}
      locations={locations}
      reminders={loaded.reminders}
      onChanged={onChanged}
    />
  );

  let body: React.ReactNode;
  if (search) {
    body = children;
  } else if (view.kind === "filter") {
    const multi = loaded.filterGroups.length > 1;
    body = loaded.filterGroups.map((group, i) => (
      // biome-ignore lint/suspicious/noArrayIndexKey: comma groups have no id; their order is the query's.
      <div key={`fg-${i}`}>
        {multi ? (
          <div className="section-title">
            <span className="mono">{group.query}</span>
            <span className="muted">{group.tasks.length}</span>
          </div>
        ) : null}
        {list(group.tasks)}
      </div>
    ));
  } else if (view.kind === "today" || view.kind === "upcoming") {
    const groups = dateGroups(view.kind === "today" ? "Today" : "Upcoming", loaded.tasks, today);
    body = groups.map((group, i) => (
      // biome-ignore lint/suspicious/noArrayIndexKey: date groups are positional (Overdue, then days).
      <div key={i}>
        {group.title !== null ? (
          <div className="section-title">
            {group.title}
            <span className="muted">{group.muted}</span>
          </div>
        ) : null}
        {list(group.tasks)}
      </div>
    ));
  } else {
    body = (
      <>
        {list(unsectioned)}
        {projectSections.map((section) => (
          <SectionBlock
            key={section.id}
            section={section}
            tasks={loaded.tasks.filter((t) => inSection(t, section))}
            all={loaded.all}
            today={today}
            reminders={loaded.reminders}
            onChanged={onChanged}
          />
        ))}
        {projectId !== null ? (
          <AddSection key={`add-section-${projectId}`} projectId={projectId} onChanged={onChanged} />
        ) : null}
      </>
    );
  }

  return (
    <main className={motion === "push" ? "main view-push" : "main"}>
      <div className={contentClass(motion, swap)}>
        <div className="view-head">
          <button
            type="button"
            className="icon-btn menu-btn"
            title="Open menu"
            aria-label="Open menu"
            onClick={onOpenDrawer}
          >
            <MenuIcon />
          </button>
          <h1 className="view-title">{viewTitle(view)}</h1>
          {toggle ? (
            <button
              type="button"
              className="view-toggle"
              title="Completed tasks are hidden by default"
              onClick={onToggleCompleted}
            >
              <CheckCircleIcon />
              {showCompleted ? "Hide completed" : "Show completed"}
            </button>
          ) : null}
        </div>
        {subtitle ? (
          <p className={view.kind === "filter" ? "view-sub mono" : "view-sub"}>{subtitle}</p>
        ) : null}
        {loaded.filterError !== null ? (
          <div className="filter-problem big-problem">{`This filter can’t run: ${loaded.filterError}`}</div>
        ) : empty && projectSections.length === 0 && !search ? (
          <div className="empty">
            <div className="big">{big}</div>
            <div>{small}</div>
          </div>
        ) : null}
        {body}
        {showCompleted && toggle && !search ? (
          <>
            <div className="section-title">
              Completed
              <span className="muted">{loaded.completed.length}</span>
            </div>
            {loaded.completed.length === 0 ? <p className="view-sub">No completed tasks here yet.</p> : null}
            {loaded.completed.map((task) => (
              <div key={`done-${task.id}`} className="task-card">
                <CompletedRow
                  task={task}
                  today={today}
                  location={locations === null ? null : (locations.get(task.id) ?? "")}
                  onChanged={onChanged}
                />
              </div>
            ))}
          </>
        ) : null}
      </div>
    </main>
  );
}
