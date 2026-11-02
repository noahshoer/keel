// The main pane for every view: the markup and branches of the Dioxus
// `MainPane` (crates/ui/src/main.rs). Dated views split into keel-core's
// date groups, filters into their comma groups, projects into sections.
// Each list ends with its "+ Add task" row, as in Dioxus.

import type { Loaded } from "../app/load";
import {
  canShowCompleted,
  emptyText,
  type Motion,
  showsProject,
  viewKey,
  viewMode,
  viewSubtitle,
  viewTitle,
} from "../app/view";
import type { Section, Task } from "../bindings";
import { localToday } from "../clock";
import { blankValues, cards, dateGroups } from "../core";
import { AddTask } from "../editor/AddTask";
import { CardList } from "../lists/CardList";
import { CompletedRow } from "../lists/CompletedRow";
import { INBOX_LOCATION, locationsOf, locationText } from "../lists/rowLayout";
import { CheckCircleIcon, ChevronLeftIcon, MenuIcon, SearchIcon } from "../shell/icons";
import { currentPlatform, searchTooltip } from "../shell/platform";
import { AddSection, SectionBlock } from "./SectionBlock";
import { useTitleFit } from "./titleFit";

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
  onLeaveSearch,
  onOpenSearch,
  motion,
  swap,
  onChanged,
  children,
}: {
  loaded: Loaded;
  showCompleted: boolean;
  onToggleCompleted: () => void;
  onOpenDrawer: () => void;
  /** The phone's Search screen goes back to the view before it. */
  onLeaveSearch: () => void;
  onOpenSearch: () => void;
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
  const fullTitle = viewTitle(view);
  const title = useTitleFit(fullTitle);
  const subtitle = viewSubtitle(view, today);
  const locations = locationsOf([...loaded.all, ...loaded.completed], loaded.projects, loaded.sections);
  const namesProject = showsProject(view);
  const search = view.kind === "search";
  const projectId = view.kind === "project" ? view.id : view.kind === "inbox" ? loaded.inbox.id : null;
  const projectSections: Section[] =
    projectId === null ? [] : loaded.sections.filter((s) => s.project_id === projectId);
  const inSection = (t: Task, s: Section) => t.section_id === s.id;
  const unsectioned = loaded.tasks.filter((t) => !projectSections.some((s) => inSection(t, s)));
  const empty = loaded.tasks.length === 0;
  const [big, small] = emptyText(view);
  // What "+ Add task" starts with: the view's project (else the Inbox), today
  // in dated views, the label in a label view. Keyed by the view, so a new
  // view starts with a closed row.
  const addRow = (
    <AddTask
      key={`add-${viewKey(view)}`}
      main
      defaults={blankValues(
        projectId ?? loaded.inbox.id,
        null,
        view.kind === "today" || view.kind === "upcoming" ? today : "",
        view.kind === "label" ? [view.name] : [],
      )}
      projects={loaded.projects}
      sections={loaded.sections}
      labels={loaded.labels}
      onChanged={onChanged}
    />
  );
  const list = (tasks: Task[]) => (
    <CardList
      cards={cards(tasks, loaded.all, mode)}
      today={today}
      locations={locations}
      labels={loaded.labels}
      reminders={loaded.reminders}
      onChanged={onChanged}
    />
  );

  let body: React.ReactNode;
  if (search) {
    body = children;
  } else if (view.kind === "filter") {
    const multi = loaded.filterGroups.length > 1;
    const groups = loaded.filterGroups.map((group, i) => (
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
    body = (
      <>
        {groups}
        {addRow}
      </>
    );
  } else if (view.kind === "today" || view.kind === "upcoming") {
    const groups = dateGroups(view.kind === "today" ? "Today" : "Upcoming", loaded.tasks, today).map(
      (group, i) => (
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
      ),
    );
    body = (
      <>
        {groups}
        {addRow}
      </>
    );
  } else {
    body = (
      <>
        {list(unsectioned)}
        {addRow}
        {projectSections.map((section) => (
          <SectionBlock
            key={section.id}
            section={section}
            tasks={loaded.tasks.filter((t) => inSection(t, section))}
            all={loaded.all}
            loaded={loaded}
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
          {search ? (
            <button type="button" className="icon-btn back-btn" aria-label="Back" onClick={onLeaveSearch}>
              <ChevronLeftIcon />
            </button>
          ) : (
            <button
              type="button"
              className="icon-btn menu-btn"
              title="Open menu"
              aria-label="Open menu"
              onClick={onOpenDrawer}
            >
              <MenuIcon />
            </button>
          )}
          <h1
            ref={title.ref}
            className="view-title"
            aria-label={title.text === fullTitle ? undefined : fullTitle}
          >
            {title.text}
          </h1>
          {toggle ? (
            <button
              type="button"
              className="view-toggle"
              title="Completed tasks are hidden by default"
              aria-label={showCompleted ? "Hide completed" : "Show completed"}
              aria-pressed={showCompleted}
              onClick={onToggleCompleted}
            >
              <CheckCircleIcon />
              <span className="view-toggle-label">{showCompleted ? "Hide completed" : "Show completed"}</span>
            </button>
          ) : null}
          {search ? null : (
            <button
              type="button"
              className="icon-btn search-btn"
              title={searchTooltip(currentPlatform())}
              aria-label="Search"
              onClick={onOpenSearch}
            >
              <SearchIcon />
            </button>
          )}
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
                  location={namesProject ? locationText(locations.get(task.id) ?? INBOX_LOCATION) : null}
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
