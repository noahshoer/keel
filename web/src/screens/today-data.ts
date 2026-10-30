// What the Today view loads, the same calls as the Dioxus App's loader
// (crates/ui/src/main.rs): the view's tasks, every task (for cards and
// sub-task counts), and the projects and sections rows are named after.
import type { Project, Section, Task } from "../bindings";
import { invoke } from "../ipc/invoke";

export type TodayData = { matched: Task[]; all: Task[]; projects: Project[]; sections: Section[] };

export async function loadToday(): Promise<TodayData> {
  const [matched, all, projects] = await Promise.all([
    invoke("today_view", {}),
    invoke("all_tasks", {}),
    invoke("list_projects", {}),
  ]);
  const sections = (
    await Promise.all(projects.map((p) => invoke("list_sections", { project_id: p.id })))
  ).flat();
  return { matched, all, projects, sections };
}
