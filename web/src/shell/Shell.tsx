// The app's frame: the sidebar and the main pane, with the markup and class
// names of crates/ui/src/main.rs (`App`) and sidebar.rs (`Sidebar`), so
// app.css applies unchanged. Read-only for now: Today is the active item,
// and nothing navigates or edits yet.
import { type ReactNode, useEffect, useState } from "react";
import type { Filter, Label, Project } from "../bindings";
import { invoke } from "../ipc/invoke";
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

type SidebarData = { projects: Project[]; labels: Label[]; filters: Filter[]; todayCount: number };

async function loadSidebar(): Promise<SidebarData> {
  const [projects, labels, filters, today] = await Promise.all([
    invoke("list_projects", {}),
    invoke("list_labels", {}),
    invoke("list_filters", {}),
    invoke("today_view", {}),
  ]);
  return { projects, labels, filters, todayCount: today.length };
}

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
  title,
}: {
  icon: ReactNode;
  iconClass: string;
  name: string;
  title?: string;
}) {
  return (
    <div className="nav-item" title={title}>
      <span className={`nav-icon ${iconClass}`}>{icon}</span>
      <span className="nav-name">{name}</span>
    </div>
  );
}

function Sidebar({ data }: { data: SidebarData | null }) {
  const day = new Date().getDate();
  const count = data && data.todayCount > 0 ? String(data.todayCount) : "";
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
      <div className="nav-item">
        <span className="nav-icon search">
          <SearchIcon />
        </span>
        Search
      </div>
      <div className="nav-item">
        <span className="nav-icon inbox">
          <InboxIcon />
        </span>
        Inbox
      </div>
      <div className="nav-item active">
        <span className="nav-icon today">
          <TodayIcon day={day} />
        </span>
        Today
        <span className="count">{count}</span>
      </div>
      <div className="nav-item">
        <span className="nav-icon upcoming">
          <UpcomingIcon />
        </span>
        Upcoming
      </div>
      <SectionHead title="My Projects" add="Add project" />
      {data?.projects
        .filter((p) => !p.is_inbox)
        .map((p) => (
          <NavRow key={p.id} icon="#" iconClass="hash" name={p.name} />
        ))}
      <SectionHead title="Labels" add="Add label" />
      {data?.labels.map((l) => (
        <NavRow key={l.id} icon={<TagIcon />} iconClass="label" name={l.name} />
      ))}
      <SectionHead title="Filters" add="Add filter" />
      {data?.filters.map((f) => (
        <NavRow key={f.id} icon={<FunnelIcon />} iconClass="filter" name={f.name} title={f.query} />
      ))}
    </aside>
  );
}

export function Shell({ children }: { children: ReactNode }) {
  const [data, setData] = useState<SidebarData | null>(null);
  useEffect(() => {
    let live = true;
    loadSidebar().then(
      (d) => {
        if (live) setData(d);
      },
      (e: unknown) => console.error("sidebar load failed", e),
    );
    return () => {
      live = false;
    };
  }, []);
  return (
    <div className="app">
      <Sidebar data={data} />
      <main className="main">{children}</main>
    </div>
  );
}
