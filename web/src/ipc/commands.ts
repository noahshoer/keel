import type { Commands } from "../bindings/Commands";

// Every Tauri command's args and return type come from Rust
// (crates/src-tauri/tests/commands_list.rs writes bindings/Commands.ts), so
// a changed return shape fails tsc.
export type { Commands };

export type CommandName = keyof Commands;

/** Runtime list of the commands; the mapped type makes it exhaustive. */
export const COMMAND_TYPES: { [K in CommandName]: true } = {
  all_tasks: true,
  check_filter: true,
  completed_in_project: true,
  completed_today: true,
  completed_with_label: true,
  diag_enabled: true,
  diag_log: true,
  dispatch: true,
  filter_view: true,
  get_reminder_settings: true,
  import_todoist_csv: true,
  label_view: true,
  list_filters: true,
  list_labels: true,
  list_projects: true,
  list_reminders: true,
  list_sections: true,
  list_tasks: true,
  notification_permission: true,
  project_view: true,
  reminders_when_closed: true,
  request_notification_permission: true,
  search: true,
  set_diag_enabled: true,
  set_reminder_settings: true,
  test_notification: true,
  today_view: true,
  upcoming_reminders: true,
  upcoming_view: true,
};

export type Handlers = {
  [K in CommandName]: (args: Commands[K]["args"]) => Promise<Commands[K]["ret"]>;
};
