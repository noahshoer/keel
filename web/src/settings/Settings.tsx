// The Settings window (the sidebar's gear): grouped cards of notification
// preferences, the completion mark, the Todoist import and diagnostics
// logging. A full screen on the phone, a dialog on desktop. Notifications are
// per device, so these never sync.
import { type ReactNode, useContext, useEffect, useRef, useState } from "react";
import type { MenuKind, PlannedReminder, ReminderSettings, SettingMenu } from "../bindings";
import { localToday } from "../clock";
import { importLine, pickSetting, reminderMenus, upcomingWhen } from "../core";
import * as diag from "../diag";
import { Menu } from "../editor/Menu";
import type { Pop } from "../editor/pop";
import { installPan } from "../gestures/pan";
import { NARROW_MAX } from "../gestures/sheet-rules";
import { invoke } from "../ipc/invoke";
import { COMPLETION_ICONS, type CompletionIcon, iconLabel, saveIcon } from "../lists/completion";
import { ShowSubtasksContext, saveShowSubtasks } from "../lists/showSubtasks";
import {
  BellSmallIcon,
  CalendarSmallIcon,
  CheckSmallIcon,
  ChevronLeftIcon,
  ClockSmallIcon,
  DoneMark,
  MonitorIcon,
  NoDateIcon,
  SunIcon,
  XIcon,
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
  const subtasks = useContext(ShowSubtasksContext);
  const [message, setMessage] = useState<string | null>(null);
  const [open, setOpen] = useState<Pop>(null);
  const modal = useRef<HTMLDivElement>(null);

  const reloadUpcoming = () => upcomingInto(setUpcoming);

  // Touch: a swipe right slides Settings away, back to the sidebar under it.
  // On the phone the screen slides off to the right, as the drawer slid in
  // (the sidebar under it is already open).
  const [leaving, setLeaving] = useState(false);
  const close = () => {
    if (leaving) return;
    if (!window.matchMedia?.("(max-width: 899px)").matches) return onClose();
    setLeaving(true);
    setTimeout(onClose, 260);
  };
  const closeNow = useRef(close);
  closeNow.current = close;
  useEffect(
    () =>
      installPan(document, (ctx) => {
        const screen = modal.current;
        const scrim = screen?.parentElement;
        if (!screen || !scrim || !ctx.inSettings || ctx.blocked || ctx.width > NARROW_MAX) return null;
        const range = screen.offsetWidth;
        return {
          dir: 1,
          range,
          onStart: () => {
            screen.style.transition = "none";
            scrim.style.transition = "none";
          },
          onMove: (p) => {
            screen.style.transform = `translateX(${p * range}px)`;
            scrim.style.opacity = String(1 - p);
          },
          // Carried on, the leaving animation takes over from where it is; otherwise it springs back.
          onEnd: (commit) => {
            if (commit) return closeNow.current();
            screen.style.transition = "transform .26s var(--ease)";
            scrim.style.transition = "opacity .26s var(--ease)";
            screen.style.transform = "";
            scrim.style.opacity = "";
            setTimeout(() => {
              screen.style.transition = "";
              scrim.style.transition = "";
            }, 260);
          },
        };
      }),
    [],
  );

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
      <div className={`modal-scrim${leaving ? " leaving" : ""}`} onClick={close}>
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
      className={`modal-scrim${leaving ? " leaving" : ""}`}
      onClick={close}
      onKeyDown={(e) => {
        if (e.key !== "Escape") return;
        if (open !== null) setOpen(null);
        else close();
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
          <button type="button" className="icon-btn back-btn" aria-label="Back" onClick={close}>
            <ChevronLeftIcon />
          </button>
          <button type="button" className="icon-btn modal-close" title="Close" onClick={close}>
            <XIcon />
          </button>
        </div>
        <h2 className="modal-title">Settings</h2>
        <h3 className="modal-section">Notifications</h3>
        <div className="settings-card">
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
              <div className="setting-text">
                <div className="setting-name">Default reminder</div>
                <div className="setting-hint">
                  Sent for each task with a due time. A task can add its own reminders too.
                </div>
              </div>
              {menu("offset", "Offset", "Default reminder", menus.offset, <ClockSmallIcon />)}
            </div>
            <div className="setting-row">
              <div className="setting-text">
                <div className="setting-name">Tasks without a time</div>
                <div className="setting-hint">
                  Sent on the due day, and for deadlines that day with no due time.
                </div>
              </div>
              {menu("allday", "AllDay", "Tasks without a time", menus.all_day, <CalendarSmallIcon />)}
            </div>
            <div className="setting-row">
              <div className="setting-text">
                <div className="setting-name">Daily summary</div>
                <div className="setting-hint">Send how many tasks are due that day at this time.</div>
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
                {enabled
                  ? "No reminders in the next two weeks."
                  : "Turn on reminders to see what's coming up."}
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
        </div>
        <h3 className="modal-section">Appearance</h3>
        <div className="settings-card">
          <CompletionSetting icon={icon} setIcon={setIcon} />
          <div className="setting-row">
            <div className="setting-text">
              <div className="setting-name">Show sub-tasks in lists</div>
              <div className="setting-hint">Off shows a task with its sub-task count</div>
            </div>
            <Switch
              on={subtasks.show}
              title="Show sub-tasks in lists"
              onClick={() => {
                saveShowSubtasks(!subtasks.show);
                subtasks.setShow(!subtasks.show);
              }}
            />
          </div>
        </div>
        <h3 className="modal-section">Import</h3>
        <div className="settings-card">
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
        </div>
        <h3 className="modal-section">Diagnostics</h3>
        <div className="settings-card">
          <div className="setting-row">
            <div className="setting-text">
              <div className="setting-name">Diagnostics logging</div>
              <div className="setting-hint">Writes a log for bug reports. Off by default.</div>
            </div>
            <Switch on={diagOn} title="Diagnostics logging" onClick={toggleDiag} />
          </div>
        </div>
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
