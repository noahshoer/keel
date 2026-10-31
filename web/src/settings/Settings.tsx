// The Settings window (the sidebar's gear), with the markup and class names
// of crates/ui/src/settings.rs so app.css applies unchanged: notification
// preferences, the completion mark, diagnostics logging and the Todoist
// import. Notifications are per device, so these never sync.
import { type ReactNode, useEffect, useRef, useState } from "react";
import type { MenuKind, PlannedReminder, ReminderSettings, SettingMenu } from "../bindings";
import { localToday } from "../clock";
import { importLine, pickSetting, reminderMenus, upcomingWhen } from "../core";
import * as diag from "../diag";
import { Menu } from "../editor/Menu";
import type { Pop } from "../editor/pop";
import { invoke } from "../ipc/invoke";
import { COMPLETION_ICONS, type CompletionIcon, iconLabel, saveIcon } from "../lists/completion";
import {
  BellSmallIcon,
  CalendarSmallIcon,
  CheckSmallIcon,
  ClockSmallIcon,
  DoneMark,
  MonitorIcon,
  NoDateIcon,
  SunIcon,
} from "../shell/icons";
import { deliveryNote, permissionHint, projectNameOf, turnOn } from "./rules";

const BLOCKED = "Notifications are blocked. Allow them for Keel in your system settings, then try again.";

/** Loads the coming-up list (empty when it fails). */
const upcomingInto = (set: (r: PlannedReminder[]) => void): Promise<void> =>
  invoke("upcoming_reminders", {}).then(set, () => set([]));

/** Which mark fills a task's circle when it's completed (this device). */
function CompletionSetting({
  icon,
  setIcon,
}: {
  icon: CompletionIcon;
  setIcon: (i: CompletionIcon) => void;
}) {
  return (
    <div className="setting-row completion-setting">
      <div className="setting-text">
        <div className="setting-name">Completion mark</div>
        <div className="setting-hint">Fills the circle when you complete a task</div>
      </div>
      <div className="mark-choices">
        {COMPLETION_ICONS.map((i) => (
          <button
            key={i}
            type="button"
            className={i === icon ? "mark-choice chosen" : "mark-choice"}
            title={iconLabel(i)}
            aria-pressed={i === icon}
            onClick={() => {
              saveIcon(i);
              setIcon(i);
            }}
          >
            <span className="mark-dot">
              <DoneMark icon={i} />
            </span>
            <span className="mark-name">{iconLabel(i)}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function Switch({ on, title, onClick }: { on: boolean; title: string; onClick: () => void }) {
  return (
    <button
      type="button"
      className={on ? "switch on" : "switch"}
      role="switch"
      aria-checked={on}
      title={title}
      onClick={onClick}
    >
      <span className="knob" />
    </button>
  );
}

export function Settings({
  onClose,
  onImported,
  icon,
  setIcon,
}: {
  onClose: () => void;
  /** After a batch of imports, so the lists reload. */
  onImported: () => void;
  icon: CompletionIcon;
  setIcon: (i: CompletionIcon) => void;
}) {
  const [settings, setSettings] = useState<ReminderSettings | null>(null);
  // The settings as last shown, so a pick right after another builds on it.
  const latest = useRef<ReminderSettings | null>(null);
  // One line per imported file.
  const [imports, setImports] = useState<Array<{ ok: boolean; text: string }>>([]);
  const [importing, setImporting] = useState(false);
  const [permission, setPermission] = useState("");
  const [whenClosed, setWhenClosed] = useState<boolean | null>(null);
  const [diagOn, setDiagOn] = useState(false);
  const [upcoming, setUpcoming] = useState<PlannedReminder[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [open, setOpen] = useState<Pop>(null);
  const modal = useRef<HTMLDivElement>(null);

  const reloadUpcoming = () => upcomingInto(setUpcoming);

  useEffect(() => {
    void (async () => {
      const ok = <T,>(p: Promise<T>) =>
        p.then(
          (v) => ({ v }),
          () => null,
        );
      const s = await ok(invoke("get_reminder_settings", {}));
      if (s) {
        latest.current = s.v;
        setSettings(s.v);
      }
      const p = await ok(invoke("notification_permission", {}));
      if (p) setPermission(p.v);
      const c = await ok(invoke("reminders_when_closed", {}));
      if (c) setWhenClosed(c.v);
      const d = await ok(invoke("diag_enabled", {}));
      if (d) setDiagOn(d.v);
      await upcomingInto(setUpcoming);
    })();
  }, []);

  // The dialog takes focus once it shows, so Escape reaches it.
  const shown = settings !== null;
  useEffect(() => {
    if (shown) modal.current?.focus();
  }, [shown]);

  // Shows a changed copy at once, saves it, then reloads the preview.
  const save = (next: ReminderSettings) => {
    latest.current = next;
    setSettings(next);
    void (async () => {
      try {
        await invoke("set_reminder_settings", { settings: next });
      } catch (e) {
        setMessage(String(e));
      }
      await reloadUpcoming();
    })();
  };

  if (settings === null || latest.current === null) {
    return (
      // biome-ignore lint/a11y/useKeyWithClickEvents: same markup as the Dioxus dialog; Escape works once it has loaded.
      // biome-ignore lint/a11y/noStaticElementInteractions: see above.
      <div className="modal-scrim" onClick={onClose}>
        {/* biome-ignore lint/a11y/useKeyWithClickEvents: only stops the click reaching the scrim. */}
        {/* biome-ignore lint/a11y/noStaticElementInteractions: see above. */}
        <div className="modal" onClick={(e) => e.stopPropagation()}>
          <p className="view-sub">Loading…</p>
        </div>
      </div>
    );
  }

  const enabled = settings.enabled;
  const menus = reminderMenus(settings);
  const pick = (kind: MenuKind) => (value: string) => {
    if (latest.current) save(pickSetting(latest.current, kind, value));
  };

  const toggle = () => {
    const s = latest.current;
    if (s === null) return;
    setMessage(null);
    if (s.enabled) {
      save({ ...s, enabled: false });
      return;
    }
    // Turning on asks the OS for permission first.
    void turnOn(permission, () => invoke("request_notification_permission", {})).then((r) => {
      setPermission(r.permission);
      if (r.granted && latest.current) save({ ...latest.current, enabled: true });
      else if (!r.granted) setMessage(BLOCKED);
    });
  };

  const toggleDiag = () => {
    const next = !diagOn;
    setDiagOn(next);
    invoke("set_diag_enabled", { on: next }).then(
      () => diag.setActive(next),
      () => setDiagOn(!next),
    );
  };

  const importFiles = (files: File[]) => {
    if (files.length === 0) return;
    setImporting(true);
    void (async () => {
      for (const file of files) {
        const project = projectNameOf(file.name);
        let line: { ok: boolean; text: string };
        let text: string | null = null;
        try {
          text = await file.text();
        } catch {
          text = null;
        }
        if (text === null) line = { ok: false, text: `${project}: couldn't read the file` };
        else {
          try {
            line = {
              ok: true,
              text: importLine(await invoke("import_todoist_csv", { project_name: project, text })),
            };
          } catch (e) {
            line = { ok: false, text: `${project}: ${String(e)}` };
          }
        }
        setImports((l) => [...l, line]);
      }
      setImporting(false);
      onImported();
    })();
  };

  const menu = (
    id: "offset" | "allday" | "summary",
    kind: MenuKind,
    title: string,
    m: SettingMenu,
    glyph: ReactNode,
  ) => (
    <Menu
      id={id}
      title={title}
      icon={glyph}
      text={m.text}
      items={m.items}
      selected={m.selected}
      open={open}
      setOpen={setOpen}
      onSelect={pick(kind)}
    />
  );

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus dialog; the scrim closes it.
    <div
      className="modal-scrim"
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key !== "Escape") return;
        if (open !== null) setOpen(null);
        else onClose();
      }}
    >
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: only stops the click reaching the scrim; keys go to the scrim. */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: see above. */}
      <div ref={modal} className="modal" tabIndex={-1} onClick={(e) => e.stopPropagation()}>
        {open !== null ? (
          // biome-ignore lint/a11y/useKeyWithClickEvents: Escape closes the menu through the scrim's handler.
          // biome-ignore lint/a11y/noStaticElementInteractions: a transparent backdrop, as in the editor.
          <div className="pop-backdrop" onClick={() => setOpen(null)} />
        ) : null}
        <div className="modal-head">
          <h2>Settings</h2>
          <button type="button" className="icon-btn" title="Close" onClick={onClose}>
            ×
          </button>
        </div>
        <h3 className="modal-section">Notifications</h3>
        <div className="setting-row">
          <div className="setting-text">
            <div className="setting-name">Task reminders</div>
            <div className="setting-hint">{permissionHint(permission, enabled)}</div>
          </div>
          <Switch on={enabled} title="Task reminders" onClick={toggle} />
        </div>
        {message !== null ? <div className="setting-warning">{message}</div> : null}
        <div className={enabled ? "setting-group" : "setting-group disabled"}>
          <div className="setting-row">
            <div className="setting-name">Remind me</div>
            {menu("offset", "Offset", "Remind me", menus.offset, <ClockSmallIcon />)}
          </div>
          <div className="setting-row">
            <div className="setting-name">Tasks without a time</div>
            {menu("allday", "AllDay", "Tasks without a time", menus.all_day, <CalendarSmallIcon />)}
          </div>
          <div className="setting-row">
            <div className="setting-text">
              <div className="setting-name">Daily summary</div>
              <div className="setting-hint">How many tasks are due that day</div>
            </div>
            {menu("summary", "Summary", "Daily summary", menus.summary, <SunIcon />)}
          </div>
          <div className="setting-row">
            <div className="setting-name">Coming up</div>
            <button
              type="button"
              className="btn btn-cancel"
              onClick={() => {
                invoke("test_notification", {}).catch((e: unknown) => setMessage(String(e)));
              }}
            >
              Send test notification
            </button>
          </div>
          {upcoming.length === 0 ? (
            <div className="setting-hint upcoming-empty">
              {enabled ? "No reminders in the next two weeks." : "Turn on reminders to see what's coming up."}
            </div>
          ) : (
            <ul className="upcoming-list">
              {upcoming.map((r) => (
                <li key={r.id}>
                  <span className="upcoming-when">{upcomingWhen(r.at, localToday())}</span>
                  <span className="upcoming-title">
                    {r.task_id !== null ? r.title : `Daily summary · ${r.body}`}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
        <h3 className="modal-section">Appearance</h3>
        <CompletionSetting icon={icon} setIcon={setIcon} />
        <h3 className="modal-section">Diagnostics</h3>
        <div className="setting-row">
          <div className="setting-text">
            <div className="setting-name">Diagnostics logging</div>
            <div className="setting-hint">Writes a log for bug reports. Off by default.</div>
          </div>
          <Switch on={diagOn} title="Diagnostics logging" onClick={toggleDiag} />
        </div>
        <h3 className="modal-section">Import</h3>
        <div className="setting-row">
          <div className="setting-text">
            <div className="setting-name">From Todoist</div>
            <div className="setting-hint">
              In Todoist, open a project → ⋯ → Manage data → Export as CSV. Each file becomes a new project.
            </div>
          </div>
          <label className={importing ? "btn btn-cancel file-btn busy" : "btn btn-cancel file-btn"}>
            {importing ? "Importing…" : "Choose CSV files"}
            <input
              type="file"
              accept=".csv,text/csv"
              multiple
              disabled={importing}
              onChange={(e) => importFiles([...(e.currentTarget.files ?? [])])}
            />
          </label>
        </div>
        {imports.length > 0 ? (
          <ul className="import-results">
            {imports.map((l, i) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: lines are only ever appended.
              <li key={i} className={l.ok ? "ok" : "bad"}>
                {l.ok ? <CheckSmallIcon /> : <NoDateIcon />}
                <span>{l.text}</span>
              </li>
            ))}
          </ul>
        ) : null}
        {whenClosed !== null ? (
          <p className="modal-foot">
            {whenClosed ? <BellSmallIcon /> : <MonitorIcon />}
            {deliveryNote(whenClosed)}
          </p>
        ) : null}
      </div>
    </div>
  );
}
