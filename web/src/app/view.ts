// Which list the main pane shows, and the screen's rules per view: title,
// subtitle, empty text, motion, and which keel-core card mode it asks for
// (crates/ui/src/main.rs `View`, `view_mode`, `view_motion`, `MainPane`).
// Picking a mode decides nothing about a task; the grouping is keel-core's.
import type { CardMode, Id } from "../bindings";
import { daySubtitle } from "../clock";

export type View =
  | { kind: "inbox" }
  | { kind: "today" }
  | { kind: "upcoming" }
  | { kind: "project"; id: Id; name: string }
  | { kind: "label"; name: string }
  | { kind: "filter"; id: Id; name: string; query: string }
  | { kind: "search" };

export type Motion = "fade" | "push" | "pop";

/** How many days Upcoming shows. */
export const UPCOMING_DAYS = 14;

/** A stable string for a view (React keys, equality). */
export function viewKey(v: View): string {
  switch (v.kind) {
    case "project":
      return `project:${v.id}:${v.name}`;
    case "label":
      return `label:${v.name}`;
    case "filter":
      return `filter:${v.id}:${v.name}:${v.query}`;
    default:
      return v.kind;
  }
}

export function sameView(a: View, b: View): boolean {
  return viewKey(a) === viewKey(b);
}

/** Project views show top-level tasks; the others group every match under its top-level parent. */
export function viewMode(v: View): CardMode {
  return v.kind === "inbox" || v.kind === "project" ? "TopLevel" : "Grouped";
}

/** Search slides in over the page and back out, like an iOS push / pop; everything else fades. */
export function viewMotion(from: View, to: View): Motion {
  const a = from.kind === "search";
  const b = to.kind === "search";
  if (!a && b) return "push";
  if (a && !b) return "pop";
  return "fade";
}

export function viewTitle(v: View): string {
  switch (v.kind) {
    case "today":
      return "Today";
    case "upcoming":
      return "Upcoming";
    case "inbox":
      return "Inbox";
    case "search":
      return "Search";
    default:
      return v.name;
  }
}

export function viewSubtitle(v: View, now: Date = new Date()): string {
  switch (v.kind) {
    case "today":
      return daySubtitle(now);
    case "upcoming":
      return `Next ${UPCOMING_DAYS} days`;
    case "filter":
      return v.query;
    default:
      return "";
  }
}

/** Upcoming and filters have no completed list (as in Todoist). */
export function canShowCompleted(v: View): boolean {
  return v.kind !== "upcoming" && v.kind !== "filter";
}

/** Views that mix projects name each row's project. */
export function showsProject(v: View): boolean {
  return v.kind === "today" || v.kind === "upcoming" || v.kind === "label" || v.kind === "filter";
}

export function emptyText(v: View): [string, string] {
  switch (v.kind) {
    case "today":
      return ["You're all done for today!", "Enjoy the rest of your day."];
    case "upcoming":
      return ["Nothing scheduled.", "Tasks with dates in the next two weeks show up here."];
    case "filter":
      return ["No tasks match this filter.", "Edit it from the sidebar to widen the query."];
    default:
      return ["No tasks yet.", "Add one below."];
  }
}
