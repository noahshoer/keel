// The Search view: a query box with live results (open tasks, plus matching
// completed ones when "Show completed" is on). Results load here rather than
// in the app-wide loader, so typing doesn't reload everything
// (crates/ui/src/search.rs).
import { useEffect, useMemo, useRef, useState } from "react";
import type { Loaded } from "../app/load";
import type { SearchResults, SearchScope, Task } from "../bindings";
import { filterReading, progress } from "../core";
import { invoke } from "../ipc/invoke";
import { CompletedRow } from "../lists/CompletedRow";
import { INBOX_LOCATION, locationsOf, locationText } from "../lists/rowLayout";
import { TaskRow } from "../lists/TaskRow";
import { SearchIcon, XSmallIcon } from "../shell/icons";
import { currentPlatform, searchHint, useWideLayout } from "../shell/platform";

type Found = { query: string; results: SearchResults | null; error: string | null };

export function SearchPane({
  query,
  setQuery,
  scope,
  setScope,
  today,
  showCompleted,
  loaded,
  reloadKey,
  onChanged,
}: {
  query: string;
  setQuery: (q: string) => void;
  /** The project or label the search stays inside, if any. */
  scope: SearchScope | null;
  setScope: (s: SearchScope | null) => void;
  today: string;
  showCompleted: boolean;
  loaded: Loaded;
  reloadKey: number;
  onChanged: () => void;
}) {
  const wide = useWideLayout();
  const [found, setFound] = useState<Found>({ query: "", results: null, error: null });
  const latest = useRef(0);
  // A project or label that's gone leaves nothing to stay inside.
  const scopeName = scopeLabel(scope, loaded);
  const live = scopeName === null ? null : scope;
  useEffect(() => {
    void reloadKey;
    const mine = ++latest.current;
    if (!query.trim()) return;
    invoke("search", { query, scope: live }).then(
      (results) => {
        if (latest.current === mine) setFound({ query, results, error: null });
      },
      (e: unknown) => {
        if (latest.current === mine) setFound({ query, results: null, error: String(e) });
      },
    );
  }, [query, live, reloadKey]);

  // keel-core's reading of the query: when it isn't a filter, the backend searches its words instead.
  const { projects, sections, labels } = loaded;
  const problem = useMemo(
    () => (query.trim() === "" ? null : filterReading(query, today, projects, sections, labels).problem),
    [query, today, projects, sections, labels],
  );
  const locations = locationsOf(
    [...(found.results?.open ?? []), ...(found.results?.completed ?? [])],
    loaded.projects,
    loaded.sections,
  );
  // Touch: scrolling the results closes the keyboard, as there's no bar to close it from.
  useEffect(() => {
    const onMove = (e: TouchEvent) => {
      const field = document.querySelector<HTMLInputElement>(".search-input");
      if (field !== null && document.activeElement === field && e.target !== field) field.blur();
    };
    document.addEventListener("touchmove", onMove, { passive: true });
    return () => document.removeEventListener("touchmove", onMove);
  }, []);
  const where = (t: Task) => locations.get(t.id) ?? INBOX_LOCATION;
  // The last results stay up while a newer query loads (as Dioxus'
  // use_resource keeps its value); "Searching…" only before the first one.
  const current = found.results === null && found.error === null ? null : found;
  let results: React.ReactNode;
  if (!query.trim()) {
    results = (
      <div className="empty">
        <div className="big">Search your tasks</div>
        <div>{searchHint(currentPlatform())}</div>
      </div>
    );
  } else if (current === null) {
    results = <p className="view-sub">Searching…</p>;
  } else if (current.error !== null || current.results === null) {
    results = <div className="error">{`Search failed: ${current.error}`}</div>;
  } else {
    const r = current.results;
    const none = r.open.length === 0 && (!showCompleted || r.completed.length === 0);
    results = (
      <>
        {none ? (
          <div className="empty">
            <div className="big">{`No tasks match “${query}”`}</div>
            {!showCompleted && r.completed.length > 0 ? (
              <div>{`${r.completed.length} completed task(s) match — turn on Show completed to see them.`}</div>
            ) : (
              <div>Try fewer or shorter words.</div>
            )}
          </div>
        ) : (
          <p className="view-sub">
            {`${r.open.length === 1 ? "1 result" : `${r.open.length} results`}${wide ? ` for ${query}` : ""}`}
          </p>
        )}
        {r.open.length > 0 ? (
          <div className="task-card">
            {r.open.map((task) => (
              <TaskRow
                key={task.id}
                row={{ task, progress: progress(task.id, loaded.all, today) }}
                today={today}
                location={where(task)}
                labels={loaded.labels}
                reminders={loaded.reminders}
                onChanged={onChanged}
              />
            ))}
          </div>
        ) : null}
        {showCompleted && r.completed.length > 0 ? (
          <>
            <div className="section-title">
              Completed
              <span className="muted">{r.completed.length}</span>
            </div>
            <div className="task-card">
              {r.completed.map((task) => (
                <CompletedRow
                  key={`done-${task.id}`}
                  task={task}
                  today={today}
                  location={locationText(where(task))}
                  onChanged={onChanged}
                />
              ))}
            </div>
          </>
        ) : null}
      </>
    );
  }

  return (
    <>
      {wide ? null : (
        <div className="search-box">
          <span className="search-icon">
            <SearchIcon />
          </span>
          <input
            className="search-input"
            type="search"
            placeholder="Search tasks, descriptions and labels"
            // biome-ignore lint/a11y/noAutofocus: the Search view exists to type into (as in the Dioxus UI).
            autoFocus
            spellCheck={false}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") setQuery("");
              // The results stay; the keyboard stops covering them.
              if (e.key === "Enter") e.currentTarget.blur();
            }}
          />
          {query ? (
            <button
              type="button"
              className="search-clear"
              title="Clear"
              aria-label="Clear search"
              // Keep focus (and the keyboard) in the field.
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => setQuery("")}
            >
              <XSmallIcon />
            </button>
          ) : null}
        </div>
      )}
      {scopeName !== null ? (
        <div className="search-scope">
          <span className="muted">Searching in</span>
          <button
            type="button"
            className="scope-chip"
            title="Search everything"
            aria-label={`Searching in ${scopeName}. Remove to search everything`}
            onClick={() => setScope(null)}
          >
            {scopeName}
            <XSmallIcon />
          </button>
        </div>
      ) : null}
      {problem !== null ? (
        <p
          className="view-sub search-problem"
          role="status"
        >{`Searching for the words, not as a filter: ${problem}`}</p>
      ) : null}
      {results}
    </>
  );
}

/** The chip's text (“#Home”, “@errand”), or null when there's no scope or what it names is gone. */
function scopeLabel(scope: SearchScope | null, loaded: Loaded): string | null {
  if (scope === null) return null;
  if (scope.kind === "project") {
    const p = loaded.projects.find((x) => x.id === scope.id);
    return p === undefined ? null : `#${p.name}`;
  }
  return `@${scope.name}`;
}
