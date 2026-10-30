import type {
  ApplyResult,
  CheckFilterArgs,
  CompletedInProjectArgs,
  CompletedWithLabelArgs,
  DiagLogArgs,
  DispatchArgs,
  Filter,
  FilterGroup,
  FilterViewArgs,
  ImportSummary,
  ImportTodoistCsvArgs,
  Label,
  LabelViewArgs,
  ListSectionsArgs,
  ListTasksArgs,
  NoArgs,
  PlannedReminder,
  Project,
  ProjectViewArgs,
  Reminder,
  ReminderSettings,
  SearchArgs,
  SearchResults,
  Section,
  SetDiagEnabledArgs,
  SetReminderSettingsArgs,
  Task,
  UpcomingViewArgs,
} from "../bindings";

/**
 * Every Tauri command: its `args` type and what it resolves to. Checked
 * against `crates/src-tauri/src/lib.rs` and `diag.rs` (`Result<T, String>`
 * resolves to `T`; `()` is serialized as `null`). The `…Args` types come
 * from `crates/src-tauri/src/args.rs`.
 */
export interface Commands {
  all_tasks: { args: NoArgs; ret: Task[] };
  check_filter: { args: CheckFilterArgs; ret: null };
  completed_in_project: { args: CompletedInProjectArgs; ret: Task[] };
  completed_today: { args: NoArgs; ret: Task[] };
  completed_with_label: { args: CompletedWithLabelArgs; ret: Task[] };
  diag_enabled: { args: NoArgs; ret: boolean };
  diag_log: { args: DiagLogArgs; ret: null };
  dispatch: { args: DispatchArgs; ret: ApplyResult };
  filter_view: { args: FilterViewArgs; ret: FilterGroup[] };
  get_reminder_settings: { args: NoArgs; ret: ReminderSettings };
  import_todoist_csv: { args: ImportTodoistCsvArgs; ret: ImportSummary };
  label_view: { args: LabelViewArgs; ret: Task[] };
  list_filters: { args: NoArgs; ret: Filter[] };
  list_labels: { args: NoArgs; ret: Label[] };
  list_projects: { args: NoArgs; ret: Project[] };
  list_reminders: { args: NoArgs; ret: Reminder[] };
  list_sections: { args: ListSectionsArgs; ret: Section[] };
  list_tasks: { args: ListTasksArgs; ret: Task[] };
  notification_permission: { args: NoArgs; ret: string };
  project_view: { args: ProjectViewArgs; ret: Task[] };
  reminders_when_closed: { args: NoArgs; ret: boolean };
  request_notification_permission: { args: NoArgs; ret: string };
  search: { args: SearchArgs; ret: SearchResults };
  set_diag_enabled: { args: SetDiagEnabledArgs; ret: null };
  set_reminder_settings: { args: SetReminderSettingsArgs; ret: null };
  test_notification: { args: NoArgs; ret: null };
  today_view: { args: NoArgs; ret: Task[] };
  upcoming_reminders: { args: NoArgs; ret: PlannedReminder[] };
  upcoming_view: { args: UpcomingViewArgs; ret: Task[] };
}

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
