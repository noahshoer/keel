// The sidebar, with the markup and class names of crates/ui/src/sidebar.rs
// so app.css applies unchanged. Rows switch the view; the management forms
// come next.
import type { ReactNode } from "react";
import type { Loaded } from "../app/load";
import type { View } from "../app/view";
import {
  FunnelIcon,
  GearIcon,
  InboxIcon,
  Logo,
  MonitorIcon,
  PlusIcon,
  SearchIcon,
  TagIcon,
  TodayIcon,
  UpcomingIcon,
} from "./icons";

const navClass = (active: boolean) => (active ? "nav-item active" : "nav-item");

function SectionHead({ title, add }: { title: string; add: string }) {
  return (
    <div className="section-head">
      {title}
      <button type="button" className="icon-btn" title={add}>
        <PlusIcon size={16} />
      </button>
    </div>
  );
}

function NavRow({
  icon,
  iconClass,
  name,
  active,
  title,
  onOpen,
}: {
  icon: ReactNode;
  iconClass: string;
  name: string;
  active: boolean;
  title?: string;
  onOpen: () => void;
}) {
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: same markup as the Dioxus sidebar (div rows); keyboard nav is "/" and Cmd+K.
    // biome-ignore lint/a11y/noStaticElementInteractions: see above.
    <div className={navClass(active)} title={title} onClick={onOpen}>
      <span className={`nav-icon ${iconClass}`}>{icon}</span>
      <span className="nav-name">{name}</span>
    </div>
  );
}

export function Sidebar({
  view,
  setView,
  loaded,
}: {
  view: View;
  setView: (v: View) => void;
  loaded: Loaded;
}) {
  const day = new Date().getDate();
  const count = loaded.todayCount > 0 ? String(loaded.todayCount) : "";
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark" title="Keel">
          <Logo size={28} />
        </div>
        Keel
        <button type="button" className="icon-btn theme-toggle" title="System theme">
          <MonitorIcon />
        </button>
        <button type="button" className="icon-btn settings-btn" title="Settings">
          <GearIcon />
        </button>
      </div>
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: Dioxus markup (div rows). */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: Dioxus markup (div rows). */}
      <div className={navClass(view.kind === "search")} onClick={() => setView({ kind: "search" })}>
        <span className="nav-icon search">
          <SearchIcon />
        </span>
        Search
      </div>
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: Dioxus markup (div rows). */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: Dioxus markup (div rows). */}
      <div className={navClass(view.kind === "inbox")} onClick={() => setView({ kind: "inbox" })}>
        <span className="nav-icon inbox">
          <InboxIcon />
        </span>
        Inbox
      </div>
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: Dioxus markup (div rows). */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: Dioxus markup (div rows). */}
      <div className={navClass(view.kind === "today")} onClick={() => setView({ kind: "today" })}>
        <span className="nav-icon today">
          <TodayIcon day={day} />
        </span>
        Today
        <span className="count">{count}</span>
      </div>
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: Dioxus markup (div rows). */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: Dioxus markup (div rows). */}
      <div className={navClass(view.kind === "upcoming")} onClick={() => setView({ kind: "upcoming" })}>
        <span className="nav-icon upcoming">
          <UpcomingIcon />
        </span>
        Upcoming
      </div>
      <SectionHead title="My Projects" add="Add project" />
      {loaded.projects
        .filter((p) => !p.is_inbox)
        .map((p) => (
          <NavRow
            key={p.id}
            icon="#"
            iconClass="hash"
            name={p.name}
            active={view.kind === "project" && view.id === p.id}
            onOpen={() => setView({ kind: "project", id: p.id, name: p.name })}
          />
        ))}
      <SectionHead title="Labels" add="Add label" />
      {loaded.labels.map((l) => (
        <NavRow
          key={l.id}
          icon={<TagIcon />}
          iconClass="label"
          name={l.name}
          active={view.kind === "label" && view.name === l.name}
          onOpen={() => setView({ kind: "label", name: l.name })}
        />
      ))}
      <SectionHead title="Filters" add="Add filter" />
      {loaded.filters.map((f) => (
        <NavRow
          key={f.id}
          icon={<FunnelIcon />}
          iconClass="filter"
          name={f.name}
          title={f.query}
          active={view.kind === "filter" && view.id === f.id}
          onOpen={() => setView({ kind: "filter", id: f.id, name: f.name, query: f.query })}
        />
      ))}
    </aside>
  );
}
