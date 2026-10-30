// The sidebar, with the markup and class names of crates/ui/src/sidebar.rs
// so app.css applies unchanged: the fixed views, then projects, labels and
// filters, each with its add form and row actions. The Settings button
// opens Settings once it exists (Plan 6).
import { useRef, useState } from "react";
import type { Loaded } from "../app/load";
import type { View } from "../app/view";
import {
  addFilter,
  addLabel,
  addProject,
  deleteFilter,
  deleteLabel,
  deleteProject,
  renameLabel,
  renameProject,
  updateFilter,
} from "../ipc/actions";
import { FilterForm, FilterRow } from "./FilterForm";
import { openAndFocus } from "./focus";
import {
  GearIcon,
  InboxIcon,
  Logo,
  MonitorIcon,
  MoonIcon,
  PlusIcon,
  SearchIcon,
  SunIcon,
  TagIcon,
  TodayIcon,
  UpcomingIcon,
} from "./icons";
import { ManagedRow } from "./ManagedRow";
import { applyTheme, nextTheme, type Theme, themeLabel } from "./theme";

const navClass = (active: boolean) => (active ? "nav-item active" : "nav-item");

function SectionHead({ title, add, onAdd }: { title: string; add: string; onAdd: () => void }) {
  return (
    <div className="section-head">
      {title}
      <button type="button" className="icon-btn" title={add} onClick={onAdd}>
        <PlusIcon size={16} />
      </button>
    </div>
  );
}

/** "Name, then Enter": the add-project and add-label forms. */
function AddForm({
  placeholder,
  field,
  onAdd,
}: {
  placeholder: string;
  field: React.RefObject<HTMLInputElement | null>;
  onAdd: (name: string) => Promise<unknown>;
}) {
  const [name, setName] = useState("");
  return (
    <form
      className="project-form"
      onSubmit={(e) => {
        e.preventDefault();
        const n = name.trim();
        if (n) void onAdd(n).then(() => setName(""));
      }}
    >
      <input
        ref={field}
        className="project-input"
        placeholder={placeholder}
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
    </form>
  );
}

export function Sidebar({
  view,
  setView,
  loaded,
  theme,
  setTheme,
  onChanged,
}: {
  view: View;
  setView: (v: View) => void;
  loaded: Loaded;
  theme: Theme;
  setTheme: (t: Theme) => void;
  onChanged: () => void;
}) {
  const [adding, setAdding] = useState<"project" | "label" | "filter" | null>(null);
  const projectField = useRef<HTMLInputElement>(null);
  const labelField = useRef<HTMLInputElement>(null);
  const filterField = useRef<HTMLInputElement>(null);
  const toggle = (what: "project" | "label" | "filter", field: React.RefObject<HTMLInputElement | null>) =>
    openAndFocus(() => setAdding((a) => (a === what ? null : what)), field);
  // Every change reloads; a failed one too (the backend's state is the truth).
  const then = (p: Promise<unknown>) => p.then(onChanged, onChanged);
  const day = new Date().getDate();
  const count = loaded.todayCount > 0 ? String(loaded.todayCount) : "";
  const themeIcon = theme === "light" ? <SunIcon /> : theme === "dark" ? <MoonIcon /> : <MonitorIcon />;

  const fixed = (kind: "search" | "inbox" | "today" | "upcoming", icon: React.ReactNode, label: string) => (
    // biome-ignore lint/a11y/useKeyWithClickEvents: same markup as the Dioxus sidebar (div rows); "/" and Cmd+K open Search.
    // biome-ignore lint/a11y/noStaticElementInteractions: see above.
    <div className={navClass(view.kind === kind)} onClick={() => setView({ kind })}>
      <span className={`nav-icon ${kind}`}>{icon}</span>
      {label}
      {kind === "today" ? <span className="count">{count}</span> : null}
    </div>
  );

  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark" title="Keel">
          <Logo size={28} />
        </div>
        Keel
        <button
          type="button"
          className="icon-btn theme-toggle"
          title={themeLabel(theme)}
          onClick={() => {
            const next = nextTheme(theme);
            applyTheme(next);
            setTheme(next);
          }}
        >
          {themeIcon}
        </button>
        <button type="button" className="icon-btn settings-btn" title="Settings">
          <GearIcon />
        </button>
      </div>
      {fixed("search", <SearchIcon />, "Search")}
      {fixed("inbox", <InboxIcon />, "Inbox")}
      {fixed("today", <TodayIcon day={day} />, "Today")}
      {fixed("upcoming", <UpcomingIcon />, "Upcoming")}

      <SectionHead title="My Projects" add="Add project" onAdd={() => toggle("project", projectField)} />
      {adding === "project" ? (
        <AddForm
          placeholder="Project name, then Enter"
          field={projectField}
          onAdd={(n) => then(addProject(n)).then(() => setAdding(null))}
        />
      ) : null}
      {loaded.projects
        .filter((p) => !p.is_inbox)
        .map((p) => (
          <ManagedRow
            key={p.id}
            name={p.name}
            icon="#"
            iconClass="hash"
            active={view.kind === "project" && view.id === p.id}
            kind="project"
            menuId={`project:${p.id}`}
            onOpen={() => setView({ kind: "project", id: p.id, name: p.name })}
            onRename={(name) => {
              void then(renameProject(p.id, name));
              // Keep the open view's title in sync with the new name.
              if (view.kind === "project" && view.id === p.id) setView({ kind: "project", id: p.id, name });
            }}
            onDelete={() => {
              void then(deleteProject(p.id));
              if (view.kind === "project" && view.id === p.id) setView({ kind: "inbox" });
            }}
          />
        ))}

      <SectionHead title="Labels" add="Add label" onAdd={() => toggle("label", labelField)} />
      {adding === "label" ? (
        <AddForm
          placeholder="Label name, then Enter"
          field={labelField}
          onAdd={(n) => then(addLabel(n)).then(() => setAdding(null))}
        />
      ) : null}
      {loaded.labels.map((l) => {
        const open = view.kind === "label" && view.name.toLowerCase() === l.name.toLowerCase();
        return (
          <ManagedRow
            key={l.id}
            name={l.name}
            icon={<TagIcon />}
            iconClass="label"
            active={view.kind === "label" && view.name === l.name}
            kind="label"
            menuId={`label:${l.id}`}
            onOpen={() => setView({ kind: "label", name: l.name })}
            onRename={(name) => {
              void then(renameLabel(l.id, name));
              if (open) setView({ kind: "label", name });
            }}
            onDelete={() => {
              void then(deleteLabel(l.id));
              if (open) setView({ kind: "inbox" });
            }}
          />
        );
      })}

      <SectionHead title="Filters" add="Add filter" onAdd={() => toggle("filter", filterField)} />
      {adding === "filter" ? (
        <FilterForm
          initialName=""
          initialQuery=""
          submitText="Add filter"
          nameRef={filterField}
          onSave={(name, query) => {
            setAdding(null);
            addFilter(name, query).then((res) => {
              const id = Object.values(res.id_mapping)[0];
              if (id !== undefined) setView({ kind: "filter", id, name, query });
              onChanged();
            }, onChanged);
          }}
          onCancel={() => setAdding(null)}
        />
      ) : null}
      {loaded.filters.map((f) => (
        <FilterRow
          key={f.id}
          filter={f}
          active={view.kind === "filter" && view.id === f.id}
          onOpen={() => setView({ kind: "filter", id: f.id, name: f.name, query: f.query })}
          onSave={(name, query) => {
            void then(updateFilter(f.id, name, query));
            // Keep the open view in sync with the edited filter.
            if (view.kind === "filter" && view.id === f.id)
              setView({ kind: "filter", id: f.id, name, query });
          }}
          onDelete={() => {
            void then(deleteFilter(f.id));
            if (view.kind === "filter" && view.id === f.id) setView({ kind: "inbox" });
          }}
        />
      ))}
    </aside>
  );
}
