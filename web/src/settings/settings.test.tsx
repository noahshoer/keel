import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import type { PlannedReminder, ReminderSettings } from "../bindings";
import type { CommandName } from "../ipc/commands";
import { mockHandlers } from "../ipc/mock";
import { savedIcon } from "../lists/completion";
import { loadCoreForTests } from "../test/wasm";
import { Settings } from "./Settings";

beforeAll(() => {
  loadCoreForTests();
});

afterEach(() => {
  delete window.__TAURI__;
  localStorage.clear();
});

type Call = { cmd: string; args: Record<string, unknown> };

/** The mock backend with some commands replaced; records every call. */
function backend(
  overrides: Partial<Record<CommandName, (args: Record<string, unknown>) => Promise<unknown>>> = {},
) {
  const calls: Call[] = [];
  window.__TAURI__ = {
    core: {
      invoke: (cmd, payload) => {
        const args = (payload?.["args"] ?? {}) as Record<string, unknown>;
        calls.push({ cmd, args });
        const own = overrides[cmd as CommandName];
        if (own) return own(args);
        const h = mockHandlers[cmd as CommandName] as (a: unknown) => Promise<unknown>;
        return h(args);
      },
    },
  };
  return calls;
}

const saves = (calls: Call[]) =>
  calls.filter((c) => c.cmd === "set_reminder_settings").map((c) => c.args["settings"] as ReminderSettings);

async function open(props: Partial<Parameters<typeof Settings>[0]> = {}) {
  const onClose = vi.fn();
  const onImported = vi.fn();
  const setIcon = vi.fn();
  render(<Settings onClose={onClose} onImported={onImported} icon="wave" setIcon={setIcon} {...props} />);
  await screen.findByText("Task reminders");
  return { onClose, onImported, setIcon };
}

const reminderSwitch = () => screen.getByTitle("Task reminders");
const diagSwitch = () => screen.getByTitle("Diagnostics logging");

it("shows every section with the stored values", async () => {
  backend();
  await open();
  for (const h of ["Notifications", "Appearance", "Diagnostics", "Import"])
    expect(screen.getByText(h)).toBeTruthy();
  expect(reminderSwitch().className).toBe("switch on");
  expect(reminderSwitch().getAttribute("aria-checked")).toBe("true");
  expect(screen.getByText("On")).toBeTruthy();
  expect(screen.getByTitle("Remind me").textContent).toContain("At the due time");
  expect(screen.getByTitle("Tasks without a time").textContent).toContain("9 AM");
  expect(screen.getByTitle("Daily summary").textContent).toContain("Off");
  expect(diagSwitch().getAttribute("aria-checked")).toBe("false");
  // The focus effect runs after the commit that shows the values, so wait for it.
  await waitFor(() => expect(document.activeElement).toBe(document.querySelector(".modal")));
  await waitFor(() =>
    expect(document.querySelector(".modal-foot")?.textContent).toBe(
      "Reminders appear while Keel is open on this computer.",
    ),
  );
});

it("the delivery note follows reminders_when_closed", async () => {
  backend({ reminders_when_closed: async () => true });
  await open();
  await waitFor(() =>
    expect(document.querySelector(".modal-foot")?.textContent).toBe(
      "Reminders arrive even when Keel is closed.",
    ),
  );
});

it("turning reminders off saves them off and greys the group", async () => {
  const calls = backend();
  await open();
  fireEvent.click(reminderSwitch());
  await waitFor(() => expect(saves(calls)).toHaveLength(1));
  expect(saves(calls)[0]?.enabled).toBe(false);
  expect(reminderSwitch().className).toBe("switch");
  expect(document.querySelector(".setting-group")?.className).toBe("setting-group disabled");
});

it("turning on while the system denies shows the warning and saves nothing", async () => {
  const calls = backend({
    get_reminder_settings: async () => ({
      enabled: false,
      offset_minutes: 0,
      all_day_time: null,
      daily_summary: null,
    }),
    notification_permission: async () => "prompt",
    request_notification_permission: async () => "denied",
  });
  await open();
  fireEvent.click(reminderSwitch());
  await screen.findByText(
    "Notifications are blocked. Allow them for Keel in your system settings, then try again.",
  );
  expect(saves(calls)).toHaveLength(0);
  expect(reminderSwitch().className).toBe("switch");
  expect(screen.getByText("Blocked by the system")).toBeTruthy();
});

it("turning on asks the system, then saves them on", async () => {
  const calls = backend({
    get_reminder_settings: async () => ({
      enabled: false,
      offset_minutes: 0,
      all_day_time: null,
      daily_summary: null,
    }),
    notification_permission: async () => "prompt",
  });
  await open();
  fireEvent.click(reminderSwitch());
  await waitFor(() => expect(saves(calls)).toHaveLength(1));
  expect(calls.some((c) => c.cmd === "request_notification_permission")).toBe(true);
  expect(saves(calls)[0]?.enabled).toBe(true);
});

it("two picks in a row each save the latest settings", async () => {
  const calls = backend();
  await open();
  fireEvent.click(screen.getByTitle("Remind me"));
  fireEvent.click(screen.getByText("5 minutes before"));
  fireEvent.click(screen.getByTitle("Daily summary"));
  fireEvent.click(screen.getByText("7:30 AM"));
  await waitFor(() => expect(saves(calls)).toHaveLength(2));
  expect(saves(calls)[1]).toEqual({
    enabled: true,
    offset_minutes: 5,
    all_day_time: "09:00:00",
    daily_summary: "07:30:00",
  });
  expect(screen.getByTitle("Daily summary").textContent).toContain("7:30 AM");
});

it("a failed save shows its error", async () => {
  backend({ set_reminder_settings: () => Promise.reject("disk full") });
  await open();
  fireEvent.click(screen.getByTitle("Tasks without a time"));
  fireEvent.click(screen.getByText("No reminder"));
  await screen.findByText("disk full");
});

it("sends a test notification", async () => {
  const calls = backend();
  await open();
  fireEvent.click(screen.getByText("Send test notification"));
  await waitFor(() => expect(calls.some((c) => c.cmd === "test_notification")).toBe(true));
});

it("lists what's coming up, the daily summary by its body", async () => {
  const at = (d: Date) => {
    const p = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T17:00:00`;
  };
  const upcoming: PlannedReminder[] = [
    { id: 1, task_id: "t1", at: at(new Date()), title: "Call mom", body: "" },
    { id: 2, task_id: null, at: at(new Date()), title: "Keel", body: "3 tasks due today" },
  ];
  backend({ upcoming_reminders: async () => upcoming });
  await open();
  const rows = await waitFor(() => {
    const r = document.querySelectorAll(".upcoming-list li");
    expect(r).toHaveLength(2);
    return r;
  });
  expect(rows[0]?.querySelector(".upcoming-when")?.textContent).toBe("Today 5 PM");
  expect(rows[0]?.querySelector(".upcoming-title")?.textContent).toBe("Call mom");
  expect(rows[1]?.querySelector(".upcoming-title")?.textContent).toBe("Daily summary · 3 tasks due today");
});

it("the empty list says why", async () => {
  backend();
  await open();
  expect(screen.getByText("No reminders in the next two weeks.")).toBeTruthy();
  fireEvent.click(reminderSwitch());
  await screen.findByText("Turn on reminders to see what's coming up.");
});

it("a completion mark choice is stored and passed up", async () => {
  backend();
  const { setIcon } = await open();
  const choices = document.querySelectorAll(".mark-choice");
  expect([...choices].map((c) => c.textContent)).toEqual(["Waterline", "Pine", "Pine + reflection"]);
  expect(choices[0]?.className).toBe("mark-choice chosen");
  fireEvent.click(screen.getByTitle("Pine"));
  expect(setIcon).toHaveBeenCalledWith("pine");
  expect(savedIcon()).toBe("pine");
});

it("the diag switch saves, and flips back when that fails", async () => {
  const calls = backend();
  await open();
  fireEvent.click(diagSwitch());
  await waitFor(() =>
    expect(calls.some((c) => c.cmd === "set_diag_enabled" && c.args["on"] === true)).toBe(true),
  );
  expect(diagSwitch().getAttribute("aria-checked")).toBe("true");
  // Back off, so the page's logging hooks aren't left on for other tests.
  fireEvent.click(diagSwitch());
  await waitFor(() => expect(diagSwitch().getAttribute("aria-checked")).toBe("false"));
});

it("a failed diag switch flips back", async () => {
  backend({ set_diag_enabled: () => Promise.reject("no") });
  await open();
  fireEvent.click(diagSwitch());
  expect(diagSwitch().getAttribute("aria-checked")).toBe("true");
  await waitFor(() => expect(diagSwitch().getAttribute("aria-checked")).toBe("false"));
});

it("imports each file in order, one line each, then reloads once", async () => {
  const calls = backend({
    import_todoist_csv: async (args) => {
      if (args["project_name"] === "Bad") return Promise.reject("not a Todoist CSV export");
      return {
        project_id: "p",
        project_name: args["project_name"],
        tasks: 2,
        sections: 1,
        labels_created: 0,
        comments: 0,
        kept_as_text: 0,
      };
    },
  });
  const { onImported } = await open();
  const input = document.querySelector<HTMLInputElement>(".file-btn input");
  expect(input?.accept).toBe(".csv,text/csv");
  expect(input?.multiple).toBe(true);
  const files = [new File(["TYPE,CONTENT\n"], "Travel.csv"), new File(["x"], "Bad.csv")];
  await act(async () => {
    fireEvent.change(input as HTMLInputElement, { target: { files } });
  });
  await waitFor(() => expect(document.querySelectorAll(".import-results li")).toHaveLength(2));
  const lines = [...document.querySelectorAll(".import-results li")];
  expect(lines.map((l) => [l.className, l.textContent])).toEqual([
    ["ok", "Travel: 2 tasks, 1 section"],
    ["bad", "Bad: not a Todoist CSV export"],
  ]);
  const sent = calls.filter((c) => c.cmd === "import_todoist_csv");
  expect(sent.map((c) => c.args["project_name"])).toEqual(["Travel", "Bad"]);
  expect(sent[0]?.args["text"]).toBe("TYPE,CONTENT\n");
  expect(onImported).toHaveBeenCalledTimes(1);
  expect(screen.getByText("Choose CSV files")).toBeTruthy();
});

it("Escape closes an open menu first, then the dialog", async () => {
  backend();
  const { onClose } = await open();
  fireEvent.click(screen.getByTitle("Remind me"));
  expect(document.querySelector(".popover.menu")).toBeTruthy();
  expect(document.querySelector(".pop-backdrop")).toBeTruthy();
  const modal = document.querySelector(".modal") as HTMLElement;
  fireEvent.keyDown(modal, { key: "Escape" });
  expect(document.querySelector(".popover.menu")).toBeNull();
  expect(onClose).not.toHaveBeenCalled();
  fireEvent.keyDown(modal, { key: "Escape" });
  expect(onClose).toHaveBeenCalledTimes(1);
});

it("the scrim and the × close it; a click inside doesn't", async () => {
  backend();
  const { onClose } = await open();
  fireEvent.click(document.querySelector(".modal") as HTMLElement);
  expect(onClose).not.toHaveBeenCalled();
  fireEvent.click(within(document.querySelector(".modal-head") as HTMLElement).getByTitle("Close"));
  fireEvent.click(document.querySelector(".modal-scrim") as HTMLElement);
  expect(onClose).toHaveBeenCalledTimes(2);
});
