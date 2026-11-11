// The main pane for every view: the markup and branches of the Dioxus
// `MainPane` (crates/ui/src/main.rs). Dated views split into keel-core's
// date groups, filters into their comma groups, projects into sections.
// Each list ends with its "+ Add task" row, as in Dioxus.

import { useCallback, useContext, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Loaded } from "../app/load";
import {
  canShowCompleted,
  emptyText,
  type Motion,
  showsProject,
  type View,
  viewKey,
  viewMode,
  viewPlace,
  viewSubtitle,
  viewTitle,
} from "../app/view";
import { ArrangeBar } from "../arrange/ArrangeBar";
import { ArrangedList } from "../arrange/ArrangedList";
import { ArrangePanel } from "../arrange/ArrangePanel";
import { FilterSheet } from "../arrange/FilterSheet";
import { arrangementOf, isArranged, placeOf } from "../arrange/place";
import { SortSheet } from "../arrange/SortSheet";
import { resetSort } from "../arrange/sortEdit";
import type { Arrangement, Section, Task } from "../bindings";
import { localToday } from "../clock";
import { arrange, blankValues, cards, dateGroups, filterReading } from "../core";
import { AddTask } from "../editor/AddTask";
import { arrangeView } from "../ipc/actions";
import { CardList } from "../lists/CardList";
import { CompletedRow } from "../lists/CompletedRow";
import { DropZone } from "../lists/DropZone";
import { EmptyState } from "../lists/EmptyState";
import { LocationsContext } from "../lists/locations";
import { INBOX_LOCATION, locationsOf, locationText } from "../lists/rowLayout";
import { ShowSubtasksContext } from "../lists/showSubtasks";
import { openAndFocus } from "../shell/focus";
import {
  CheckCircleIcon,
  ChevronLeftIcon,
  FunnelIcon,
  MenuIcon,
  PlusIcon,
  SearchIcon,
  SortIcon,
} from "../shell/icons";
import { type ListContext, type PageEditing, pageActions } from "../shell/listActions";
import { currentPlatform, searchTooltip } from "../shell/platform";
import { attachHeadScroll } from "./headScroll";
import { useHeldText } from "./heldText";
import { PageEdit } from "./PageEdit";
import { AddSection, SectionBlock } from "./SectionBlock";
import { useTitleFit } from "./titleFit";
import { ViewMenu } from "./ViewMenu";

const SEARCH = viewPlace({ kind: "search" });

function contentClass(motion: Motion, swap: number): string {
  if (motion === "pop") return "content view-pop";
  if (motion === "push" || motion === "still") return "content";
  return swap % 2 === 0 ? "content view-in-a" : "content view-in-b";
}

export function MainPane({
  loaded,
  showCompleted,
  onToggleCompleted,
  onOpenDrawer,
  onLeaveSearch,
  onOpenSearch,
  setView,
  renameView,
  holdName,
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
  setView: (v: View) => void;
  renameView: (v: View) => void;
  holdName: ListContext["holdName"];
  motion: Motion;
  /** Bumped when a different view lands; alternates the fade-in. */
  swap: number;
  onChanged: () => void;
  /** The Search pane, in the Search view. */
  children?: React.ReactNode;
}) {
  const { view } = loaded;
  const today = localToday();
  const { show: showSubtasks } = useContext(ShowSubtasksContext);
  const mode = viewMode(view, showSubtasks);
  const toggle = canShowCompleted(view);
  const heldTitle = useHeldText(viewTitle(view));
  const fullTitle = heldTitle.text;
  const title = useTitleFit(fullTitle);
  const head = useRef<HTMLDivElement>(null);
  const listScroll = useRef<HTMLDivElement>(null);
  const place = viewPlace(view);
  const previous = useRef<string | null>(null);
  const left = useRef({ place, top: 0 });
  // A different view opens at the top with the full title; coming back from
  // Search returns to where the list was. Attaching draws the header from the
  // scroll position at once, so it doesn't animate into place.
  useLayoutEffect(() => {
    const list = listScroll.current;
    if (!list || !head.current) return;
    const back = previous.current === SEARCH && left.current.place === place;
    list.scrollTop = back ? left.current.top : 0;
    previous.current = place;
    const detach = attachHeadScroll(list, head.current);
    if (place === SEARCH) return detach;
    // Recorded as it scrolls: by the time a change of view lands, the new content
    // may already have clamped the position.
    const record = () => {
      left.current = { place, top: list.scrollTop };
    };
    record();
    list.addEventListener("scroll", record, { passive: true });
    return () => {
      list.removeEventListener("scroll", record);
      detach();
    };
  }, [place]);
  const heldSubtitle = useHeldText(viewSubtitle(view, today));
  const subtitle = heldSubtitle.text;
  const actions = pageActions(loaded, {
    view,
    setView,
    renameView,
    holdName,
    settle: (p) => p.then(onChanged, onChanged),
  });
  // By place, so it doesn't outlive the page it was opened on.
  const [editingPlace, setEditingPlace] = useState<string | null>(null);
  const edited = editingPlace === place && actions !== null ? actions.editing : null;
  // What's saved shows at once, in the frame the field goes.
  const editing: PageEditing | null =
    edited?.kind === "name"
      ? {
          ...edited,
          rename: (name) => {
            heldTitle.hold(name);
            edited.rename(name);
          },
        }
      : edited?.kind === "filter"
        ? {
            ...edited,
            save: (name, query) => {
              heldTitle.hold(name);
              heldSubtitle.hold(query);
              edited.save(name, query);
            },
          }
        : null;
  // The open filter or sort panel (drawn by a later task), by place, so it doesn't outlive its page.
  const [panelState, setPanelState] = useState<{ place: string; which: "filter" | "sort" } | null>(null);
  const panel = panelState?.place === place ? panelState.which : null;
  const setPanel = (which: "filter" | "sort" | null) => {
    setPreviewState(null);
    setPanelState(which ? { place, which } : null);
  };
  // The filter being typed, before it's saved; it goes with the panel or the page.
  const [previewState, setPreviewState] = useState<{ place: string; arrangement: Arrangement } | null>(null);
  const preview = previewState?.place === place && panel === "filter" ? previewState.arrangement : null;
  const closePanel = useCallback(() => {
    setPanelState(null);
    setPreviewState(null);
  }, []);
  const filterField = useRef<HTMLTextAreaElement>(null);
  const filterButton = useRef<HTMLButtonElement>(null);
  const sortButton = useRef<HTMLButtonElement>(null);
  const stopEditing = () => setEditingPlace(null);
  const titleField = useRef<HTMLInputElement>(null);
  // What the project's ••• calls to open the "Add section" field at the end of the page.
  const addSectionOpener = useRef<(() => void) | null>(null);
  const [querySlot, setQuerySlot] = useState<HTMLElement | null>(null);
  const locationsValue = useMemo(
    () => ({ projects: loaded.projects, sections: loaded.sections }),
    [loaded.projects, loaded.sections],
  );
  // A saved filter naming something that's gone (a deleted project, say) still
  // runs; the page says why that part is empty. keel-core's reading, with the
  // same names the query field checks against.
  const filterQuery = view.kind === "filter" ? subtitle : "";
  const queryProblem = useMemo(
    () =>
      filterQuery === ""
        ? null
        : filterReading(filterQuery, today, loaded.projects, loaded.sections, loaded.labels).problem,
    [filterQuery, today, loaded.projects, loaded.sections, loaded.labels],
  );
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
  // The page's saved arrangement, or the filter being typed over it.
  const arrangePlace = placeOf(view, loaded);
  const saved = arrangementOf(arrangePlace, loaded.arrangements);
  const arrangement = preview ?? saved;
  const arranged = isArranged(arrangement);
  const filterSet = arrangement.filter.trim() !== "";
  const sortSet = arrangement.groups.length > 0 || arrangement.order.length > 0;
  const headed = arranged && arrangement.groups.length > 0;
  const arrangeTasks = (tasks: Task[]) =>
    arrange(tasks, arrangement, loaded.projects, loaded.sections, loaded.labels, today);
  const whole = arranged ? arrangeTasks(loaded.tasks) : null;
  const saveArrangement = (next: Arrangement) => {
    if (arrangePlace) arrangeView(arrangePlace, next).then(onChanged, onChanged);
  };
  // A filter that matched nothing says so; the page's own empty text is for pages with no tasks.
  const nothingMatches =
    whole !== null && arrangement.filter.trim() !== "" && whole.filter_problem === null && whole.shown === 0;
  // Without headings the page keeps its own grouping and each list is shaped (filtered, ordered).
  const shape: ((tasks: Task[]) => Task[]) | null =
    arranged && !headed ? (tasks) => arrangeTasks(tasks).groups[0]?.tasks ?? [] : null;
  const list = (tasks: Task[]) => (
    <CardList
      cards={cards(tasks, loaded.all, mode, today)}
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
  } else if (nothingMatches) {
    body = (
      <>
        <p className="view-sub arrange-empty">No tasks match this filter.</p>
        {addRow}
      </>
    );
  } else if (headed && whole !== null) {
    body = (
      <>
        <ArrangedList
          groups={whole.groups}
          all={loaded.all}
          mode={mode}
          today={today}
          locations={locations}
          labels={loaded.labels}
          reminders={loaded.reminders}
          onChanged={onChanged}
        />
        {addRow}
      </>
    );
  } else if (view.kind === "filter") {
    const multi = loaded.filterGroups.length > 1;
    const groups = loaded.filterGroups.flatMap((group, i) => {
      const tasks = shape ? shape(group.tasks) : group.tasks;
      if (shape && tasks.length === 0) return [];
      return [
        // biome-ignore lint/suspicious/noArrayIndexKey: comma groups have no id; their order is the query's.
        <div key={`fg-${i}`}>
          {multi ? (
            <div className="section-title">
              <span className="mono">{group.query}</span>
              <span className="muted">{tasks.length}</span>
            </div>
          ) : null}
          {list(tasks)}
        </div>,
      ];
    });
    body = (
      <>
        {groups}
        {addRow}
      </>
    );
  } else if (view.kind === "today" || view.kind === "upcoming") {
    const groups = dateGroups(view.kind === "today" ? "Today" : "Upcoming", loaded.tasks, today).flatMap(
      (group, i) => {
        const tasks = shape ? shape(group.tasks) : group.tasks;
        if (shape && tasks.length === 0) return [];
        return [
          // biome-ignore lint/suspicious/noArrayIndexKey: date groups are positional (Overdue, then days).
          <div key={i}>
            {group.title !== null ? (
              <div className="section-title">
                {group.title}
                <span className="muted">{group.muted}</span>
              </div>
            ) : null}
            {list(tasks)}
          </div>,
        ];
      },
    );
    body = (
      <>
        {groups}
        {addRow}
      </>
    );
  } else {
    const unsectionedBlock = (
      <>
        {list(shape ? shape(unsectioned) : unsectioned)}
        {addRow}
      </>
    );
    body = (
      <>
        {projectId !== null && projectSections.length > 0 ? (
          <DropZone
            project={projectId}
            section={null}
            name={loaded.projects.find((p) => p.id === projectId)?.name ?? "Inbox"}
            onChanged={onChanged}
          >
            {unsectionedBlock}
          </DropZone>
        ) : (
          unsectionedBlock
        )}
        {projectSections.map((section) => (
          <SectionBlock
            key={section.id}
            section={section}
            tasks={loaded.tasks.filter((t) => inSection(t, section))}
            all={loaded.all}
            shape={shape}
            loaded={loaded}
            today={today}
            reminders={loaded.reminders}
            onChanged={onChanged}
          />
        ))}
        {projectId !== null ? (
          <AddSection
            key={`add-section-${projectId}`}
            projectId={projectId}
            onChanged={onChanged}
            opener={addSectionOpener}
          />
        ) : null}
      </>
    );
  }

  return (
    <LocationsContext.Provider value={locationsValue}>
      <main className={motion === "push" ? "main view-push" : "main"}>
        <div className={contentClass(motion, swap)}>
          <div ref={head} className="view-head">
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
              hidden={editing !== null}
              aria-label={title.text === fullTitle ? undefined : fullTitle}
            >
              {title.text}
            </h1>
            {editing ? (
              <PageEdit
                editing={editing}
                field={titleField}
                querySlot={querySlot}
                names={loaded}
                onClose={stopEditing}
              />
            ) : null}
            {arrangePlace !== null ? (
              <>
                <button
                  ref={filterButton}
                  type="button"
                  className={filterSet ? "icon-btn arrange-btn on" : "icon-btn arrange-btn"}
                  title="Filter"
                  aria-label="Filter"
                  aria-pressed={filterSet}
                  onClick={() => {
                    if (panel === "filter") {
                      // The button doesn't count as outside the panel, so keeping a draft that reads is done here.
                      if (preview !== null && preview.filter.trim() !== saved.filter.trim())
                        saveArrangement(preview);
                      closePanel();
                    } else openAndFocus(() => setPanel("filter"), filterField);
                  }}
                >
                  <FunnelIcon />
                </button>
                <button
                  ref={sortButton}
                  type="button"
                  className={sortSet ? "icon-btn arrange-btn on" : "icon-btn arrange-btn"}
                  title="Sort"
                  aria-label="Sort"
                  aria-pressed={sortSet}
                  onClick={() => setPanel(panel === "sort" ? null : "sort")}
                >
                  <SortIcon />
                </button>
              </>
            ) : null}
            {panel === "filter" ? (
              <FilterSheet
                pageName={viewTitle(view)}
                initial={saved.filter}
                names={loaded}
                filters={loaded.filters}
                counts={{
                  shown: whole?.shown ?? loaded.tasks.length,
                  total: whole?.total ?? loaded.tasks.length,
                }}
                anchor={filterButton}
                fieldRef={filterField}
                onDraft={(query) =>
                  setPreviewState(query === null ? null : { place, arrangement: { ...saved, filter: query } })
                }
                onSave={(query) => saveArrangement({ ...saved, filter: query })}
                onFilterAdded={onChanged}
                onClose={closePanel}
              />
            ) : null}
            {panel === "sort" ? (
              <ArrangePanel
                title="Sort"
                action={{ label: "Reset to default", run: () => saveArrangement(resetSort(arrangement)) }}
                anchor={sortButton}
                onClose={closePanel}
                layoutKey={`${arrangement.groups.length}|${arrangement.order.length}`}
              >
                <SortSheet arrangement={arrangement} onChange={saveArrangement} />
              </ArrangePanel>
            ) : null}
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
                <span className="view-toggle-label">
                  {showCompleted ? "Hide completed" : "Show completed"}
                </span>
              </button>
            ) : null}
            {actions ? (
              <ViewMenu
                warning={actions.warning}
                onEdit={() => {
                  // At the top, where the title is at its full size on the phone.
                  if (listScroll.current) listScroll.current.scrollTop = 0;
                  openAndFocus(() => setEditingPlace(place), titleField);
                }}
                onDelete={actions.remove}
                lead={
                  view.kind === "project" && !headed
                    ? {
                        label: "Add section",
                        icon: <PlusIcon size={20} />,
                        run: () => addSectionOpener.current?.(),
                      }
                    : undefined
                }
              />
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
          <div ref={listScroll} className="list-scroll">
            {/* Keyed by the view, so a view lands as it is: only later changes animate (lists/presence.ts). */}
            <div key={viewKey(view)} className="list-body">
              <ArrangeBar
                arrangement={arrangement}
                problem={whole?.filter_problem ?? null}
                onOpen={setPanel}
                onClearFilter={() => saveArrangement({ ...arrangement, filter: "" })}
                onResetSort={() => saveArrangement({ ...arrangement, groups: [], order: [] })}
              />
              {editing?.kind === "filter" ? (
                <div ref={setQuerySlot} className="view-edit-query" />
              ) : subtitle ? (
                <p className={view.kind === "filter" ? "view-sub mono" : "view-sub"}>{subtitle}</p>
              ) : null}
              {loaded.filterError === null && queryProblem !== null && editing === null ? (
                <p className="filter-problem view-problem">{queryProblem}</p>
              ) : null}
              {loaded.filterError !== null ? (
                <div className="filter-problem big-problem">{`This filter can’t run: ${loaded.filterError}`}</div>
              ) : search ? null : (
                <EmptyState
                  show={empty && projectSections.length === 0 && !nothingMatches}
                  big={big}
                  small={small}
                />
              )}
              {body}
              {/* Room under the last row of the page, so the floating add button doesn't cover it. */}
              {search ? null : <div className="fab-spacer" />}
              {showCompleted && toggle && !search ? (
                <>
                  <div className="section-title">
                    Completed
                    <span className="muted">{loaded.completed.length}</span>
                  </div>
                  {loaded.completed.length === 0 ? (
                    <p className="view-sub">No completed tasks here yet.</p>
                  ) : null}
                  {loaded.completed.map((task) => (
                    <div key={`done-${task.id}`} className="task-card">
                      <CompletedRow
                        task={task}
                        today={today}
                        location={
                          namesProject ? locationText(locations.get(task.id) ?? INBOX_LOCATION) : null
                        }
                        onChanged={onChanged}
                      />
                    </div>
                  ))}
                </>
              ) : null}
            </div>
          </div>
        </div>
      </main>
    </LocationsContext.Provider>
  );
}
