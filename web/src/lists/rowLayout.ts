// What a task row shows where. Which rows appear is about the screen; the
// facts behind them (groups, counts) are keel-core's.
import type { Id, Progress, Project, Section, Task } from "../bindings";
import { allDone } from "../core";

export type RowLocation = { project: string; section: string | null };

/** The fallback when a task's project can't be found. */
export const INBOX_LOCATION: RowLocation = { project: "Inbox", section: null };

/** The bottom of a row: labels share the project's row unless a when group needs its own. */
export function bottomLayout(hasWhen: boolean, hasLabels: boolean): "one-row" | "two-rows" {
  return hasWhen && hasLabels ? "two-rows" : "one-row";
}

export function subtaskState(p: Progress): "none" | "some" | "all" {
  if (p.done === 0) return "none";
  return allDone(p) ? "all" : "some";
}

/** The ring's circumference on Phosphor's 256 grid, radius 96. */
const RING = 2 * Math.PI * 96;

/** `stroke-dasharray` for a ring whose edge fills by the share done. */
export function ringDash(p: Progress): string {
  return `${Math.round((RING * p.done) / p.total)} ${Math.round(RING)}`;
}

/** How much of a label has to show before it counts as showing. */
const READABLE = 20;

/** How many labels have too little showing to read. The first always shows. */
export function hiddenCount(lefts: number[], limit: number): number {
  return lefts.slice(1).filter((left) => left >= limit - READABLE).length;
}

export function locationText(l: RowLocation): string {
  return l.section === null ? l.project : `${l.project} / ${l.section}`;
}

/** Each task's project and section. Falls back to the Inbox's name. */
export function locationsOf(tasks: Task[], projects: Project[], sections: Section[]): Map<Id, RowLocation> {
  const names = new Map(projects.map((p) => [p.id, p.name]));
  const sectionNames = new Map(sections.map((s) => [s.id, s.name]));
  const inbox = projects.find((p) => p.is_inbox)?.name ?? INBOX_LOCATION.project;
  return new Map(
    tasks.map((t) => [
      t.id,
      {
        project: names.get(t.project_id) ?? inbox,
        section: t.section_id === null ? null : (sectionNames.get(t.section_id) ?? null),
      },
    ]),
  );
}
