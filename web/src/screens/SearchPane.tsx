// The Search view: a query box with live results (open tasks, plus matching
// completed ones when "Show completed" is on). Results load here rather than
// in the app-wide loader, so typing doesn't reload everything
// (crates/ui/src/search.rs).
import { useEffect, useRef, useState } from "react";
import type { Loaded } from "../app/load";
import type { Project, SearchResults, Section, Task } from "../bindings";
import { invoke } from "../ipc/invoke";
import { CompletedRow } from "../lists/CompletedRow";
import { TaskRow } from "../lists/TaskRow";
import { SearchIcon, XSmallIcon } from "../shell/icons";
import { currentPlatform, searchHint } from "../shell/platform";

/** "Project" or "Project / Section". */
function locationOf(t: Task, projects: Project[], sections: Section[]): string {
  const project = projects.find((p) => p.id === t.project_id)?.name ?? "";
  const section = t.section_id === null ? undefined : sections.find((s) => s.id === t.section_id);
  return section === undefined ? project : `${project} / ${section.name}`;
}

type Found = { query: string; results: SearchResults | null; error: string | null };

export function SearchPane({
  query,
  setQuery,
  today,
  showCompleted,
  loaded,
  reloadKey,
  onChanged,
}: {
  query: string;
  setQuery: (q: string) => void;
  today: string;
  showCompleted: boolean;
  loaded: Loaded;
  reloadKey: number;
  onChanged: () => void;
}) {
  const [found, setFound] = useState<Found>({ query: "", results: null, error: null });
  const latest = useRef(0);
  useEffect(() => {
    void reloadKey;
    const mine = ++latest.current;
    if (!query.trim()) return;
    invoke("search", { query }).then(
      (results) => {
        if (latest.current === mine) setFound({ query, results, error: null });
      },
      (e: unknown) => {
        if (latest.current === mine) setFound({ query, results: null, error: String(e) });
      },
    );
  }, [query, reloadKey]);

  const where = (t: Task) => locationOf(t, loaded.projects, loaded.sections);
  const current = found.query === query ? found : null;
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
          <p className="view-sub">{r.open.length === 1 ? "1 task" : `${r.open.length} tasks`}</p>
        )}
        {r.open.map((task) => (
          <TaskRow
            key={task.id}
            row={{ task, progress: null }}
            today={today}
            location={where(task)}
            reminders={loaded.reminders}
            onChanged={onChanged}
          />
        ))}
        {showCompleted && r.completed.length > 0 ? (
          <>
            <div className="section-title">
              Completed
              <span className="muted">{r.completed.length}</span>
            </div>
            {r.completed.map((task) => (
              <CompletedRow
                key={`done-${task.id}`}
                task={task}
                today={today}
                location={where(task)}
                onChanged={onChanged}
              />
            ))}
          </>
        ) : null}
      </>
    );
  }

  return (
    <>
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
      {results}
    </>
  );
}
