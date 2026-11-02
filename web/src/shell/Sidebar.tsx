// The sidebar, with the markup and class names of crates/ui/src/sidebar.rs
// so app.css applies unchanged: the fixed views, then projects, labels and
// filters, each with its add form and row actions. The gear opens Settings.
import { Children, useMemo, useRef, useState } from "react";
import type { Loaded } from "../app/load";
import { UPCOMING_DAYS, type View } from "../app/view";
import type { Id } from "../bindings";
import { localToday } from "../clock";
import { sidebarCounts } from "../core";
import { labelTagColor } from "../editor/labelColors";
import { addFilter, addLabel, addProject } from "../ipc/actions";
import type { CompletionIcon } from "../lists/completion";
import { Settings } from "../settings/Settings";
import { FORM_TOGGLE, useCloseOnClickOutside } from "./clickOutside";
import { useDrawerKeyboard } from "./drawerKeyboard";
import { FilterForm, FilterRow } from "./FilterForm";
import { openAndFocus } from "./focus";
import {
  GearIcon,
  HashSmallIcon,
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
import { filterActions, type ListContext, labelActions, projectActions } from "./listActions";
import { ManagedRow } from "./ManagedRow";
import { currentPlatform, searchTooltip, useWideLayout } from "./platform";
import { nextTheme, type Theme, themeLabel } from "./theme";

const navClass = (active: boolean) => (active ? "nav-item active" : "nav-item");

function SectionHead({ title, add, onAdd }: { title: string; add: string; onAdd: () => void }) {
  return (
    <div className="section-head">
      {title}
      <button type="button" className="icon-btn" title={add} onClick={onAdd} {...{ [FORM_TOGGLE]: "" }}>
        <PlusIcon size={16} />
      </button>
    </div>
  );
}

/** A rounded card of sidebar rows; an empty list draws none. */
function NavCard({ children }: { children: React.ReactNode }) {
  return Children.toArray(children).length > 0 ? <div className="nav-card">{children}</div> : null;
}

/** "Name, then Enter": the add-project and add-label forms. */
function AddForm({
  placeholder,
  field,
  onAdd,
  onClose,
}: {
  placeholder: string;
  field: React.RefObject<HTMLInputElement | null>;
  onAdd: (name: string) => Promise<unknown>;
  onClose: () => void;
}) {
  const [name, setName] = useState("");
  const form = useRef<HTMLFormElement>(null);
  useCloseOnClickOutside(form, onClose);
  return (
    <form
      ref={form}
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
  icon,
  setIcon,
  searchQuery,
  setSearchQuery,
  onChanged,
}: {
  view: View;
  setView: (v: View) => void;
  loaded: Loaded;
  theme: Theme;
  /** Takes the next theme from the current one (App applies it). */
  setTheme: (next: (t: Theme) => Theme) => void;
  /** The completion mark, chosen in Settings. */
  icon: CompletionIcon;
  setIcon: (i: CompletionIcon) => void;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  onChanged: () => void;
}) {
  const wide = useWideLayout();
  const aside = useRef<HTMLElement>(null);
  const keyboard = useDrawerKeyboard(aside);
  const [showSettings, setShowSettings] = useState(false);
  const [adding, setAdding] = useState<"project" | "label" | "filter" | null>(null);
  const projectField = useRef<HTMLInputElement>(null);
  const labelField = useRef<HTMLInputElement>(null);
  const filterField = useRef<HTMLInputElement>(null);
  const toggle = (what: "project" | "label" | "filter", field: React.RefObject<HTMLInputElement | null>) =>
    openAndFocus(() => setAdding((a) => (a === what ? null : what)), field);
  // Every change reloads; a failed one too (the backend's state is the truth).
  const then = (p: Promise<unknown>) => p.then(onChanged, onChanged);
  const context: ListContext = { view, setView, settle: then };
  const day = new Date().getDate();
  // A wasm call serializes every task, and a desktop search re-renders this per keystroke.
  const today = localToday();
  const counts = useMemo(() => sidebarCounts(loaded.all, today, UPCOMING_DAYS), [loaded.all, today]);
  const ofProject = (id: Id) => counts.projects.find((c) => c.project_id === id)?.count ?? 0;
  const countOf = { inbox: ofProject(loaded.inbox.id), today: counts.today, upcoming: counts.upcoming };
  const themeIcon = theme === "light" ? <SunIcon /> : theme === "dark" ? <MoonIcon /> : <MonitorIcon />;

  const fixed = (kind: "inbox" | "today" | "upcoming", icon: React.ReactNode, label: string) => (
    // biome-ignore lint/a11y/useKeyWithClickEvents: same markup as the Dioxus sidebar (div rows); "/" and Cmd+K open Search.
    // biome-ignore lint/a11y/noStaticElementInteractions: see above.
    <div className={navClass(view.kind === kind)} onClick={() => setView({ kind })}>
      <span className={`nav-icon ${kind}`}>{icon}</span>
      {label}
      {countOf[kind] > 0 ? <span className="count">{countOf[kind]}</span> : null}
    </div>
  );

  return (
    <aside
      ref={aside}
      className="sidebar"
      style={{ "--keyboard-inset": `${keyboard}px` } as React.CSSProperties}
    >
      <div className="brand">
        <div className="brand-mark" title="Keel">
          <Logo size={36} />
        </div>
        <span className="brand-name">Keel</span>
        <button
          type="button"
          className="icon-btn theme-toggle"
          title={themeLabel(theme)}
          onClick={() => setTheme(nextTheme)}
        >
          {themeIcon}
        </button>
        <button
          type="button"
          className="icon-btn settings-btn"
          title="Settings"
          onClick={() => setShowSettings(true)}
        >
          <GearIcon />
        </button>
      </div>
      {showSettings ? (
        <Settings
          onClose={() => setShowSettings(false)}
          onImported={onChanged}
          icon={icon}
          setIcon={setIcon}
        />
      ) : null}
      <div className="sidebar-search">
        <span className="sidebar-search-icon">
          <SearchIcon />
        </span>
        {/* On a phone it's a door to the Search screen; on desktop results fill the main area. */}
        <input
          className="sidebar-search-input"
          type="search"
          placeholder="Search"
          title={searchTooltip(currentPlatform())}
          spellCheck={false}
          readOnly={!wide}
          value={wide ? searchQuery : ""}
          onClick={wide ? undefined : () => setView({ kind: "search" })}
          onFocus={wide && view.kind !== "search" ? () => setView({ kind: "search" }) : undefined}
          onChange={(e) => setSearchQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") setSearchQuery("");
          }}
        />
      </div>
      <div className="nav-card">
        {fixed("inbox", <InboxIcon />, "Inbox")}
        {fixed("today", <TodayIcon day={day} />, "Today")}
        {fixed("upcoming", <UpcomingIcon />, "Upcoming")}
      </div>

      <SectionHead title="My Projects" add="Add project" onAdd={() => toggle("project", projectField)} />
      {adding === "project" ? (
        <AddForm
          placeholder="Project name, then Enter"
          field={projectField}
          onAdd={(n) => then(addProject(n)).then(() => setAdding(null))}
          onClose={() => setAdding(null)}
        />
      ) : null}
      <NavCard>
        {loaded.projects
          .filter((p) => !p.is_inbox)
          .map((p) => {
            const actions = projectActions(p, context);
            return (
              <ManagedRow
                key={p.id}
                name={p.name}
                count={ofProject(p.id)}
                icon={<HashSmallIcon />}
                iconClass="hash"
                active={view.kind === "project" && view.id === p.id}
                warning={actions.warning}
                menuId={`project:${p.id}`}
                onOpen={() => setView({ kind: "project", id: p.id, name: p.name })}
                onRename={actions.rename}
                onDelete={actions.remove}
              />
            );
          })}
      </NavCard>

      <SectionHead title="Labels" add="Add label" onAdd={() => toggle("label", labelField)} />
      {adding === "label" ? (
        <AddForm
          placeholder="Label name, then Enter"
          field={labelField}
          onAdd={(n) => then(addLabel(n)).then(() => setAdding(null))}
          onClose={() => setAdding(null)}
        />
      ) : null}
      <NavCard>
        {loaded.labels.map((l) => {
          const actions = labelActions(l, context);
          return (
            <ManagedRow
              key={l.id}
              name={l.name}
              icon={
                <span className="nav-tag" style={{ color: labelTagColor(l.color) }}>
                  <TagIcon />
                </span>
              }
              iconClass="label"
              active={view.kind === "label" && view.name === l.name}
              warning={actions.warning}
              menuId={`label:${l.id}`}
              onOpen={() => setView({ kind: "label", name: l.name })}
              onRename={actions.rename}
              onDelete={actions.remove}
            />
          );
        })}
      </NavCard>

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
      <NavCard>
        {loaded.filters.map((f) => {
          const actions = filterActions(f, context);
          return (
            <FilterRow
              key={f.id}
              filter={f}
              active={view.kind === "filter" && view.id === f.id}
              warning={actions.warning}
              onOpen={() => setView({ kind: "filter", id: f.id, name: f.name, query: f.query })}
              onSave={actions.save}
              onDelete={actions.remove}
            />
          );
        })}
      </NavCard>
    </aside>
  );
}
