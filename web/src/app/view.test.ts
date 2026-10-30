import { expect, it } from "vitest";
import {
  canShowCompleted,
  emptyText,
  sameView,
  showsProject,
  type View,
  viewMode,
  viewMotion,
  viewSubtitle,
  viewTitle,
} from "./view";

const today: View = { kind: "today" };
const inbox: View = { kind: "inbox" };
const search: View = { kind: "search" };
const upcoming: View = { kind: "upcoming" };
const project: View = { kind: "project", id: "p", name: "Home" };
const label: View = { kind: "label", name: "home" };
const filter: View = { kind: "filter", id: "f", name: "Important", query: "p1" };

it("search pushes in and pops out; everything else fades", () => {
  expect(viewMotion(today, search)).toBe("push");
  expect(viewMotion(search, inbox)).toBe("pop");
  expect(viewMotion(today, inbox)).toBe("fade");
  expect(viewMotion(search, search)).toBe("fade");
});

it("project views are top level; dated views group", () => {
  expect(viewMode(inbox)).toBe("TopLevel");
  expect(viewMode(project)).toBe("TopLevel");
  for (const v of [today, upcoming, label, filter, search]) expect(viewMode(v)).toBe("Grouped");
});

it("titles, subtitles and what each view shows", () => {
  expect([today, upcoming, inbox, project, label, filter, search].map(viewTitle)).toEqual([
    "Today",
    "Upcoming",
    "Inbox",
    "Home",
    "home",
    "Important",
    "Search",
  ]);
  const fri = new Date(2026, 9, 30);
  expect(viewSubtitle(today, fri)).toBe("Friday 30 Oct");
  expect(viewSubtitle(upcoming, fri)).toBe("Next 14 days");
  expect(viewSubtitle(filter, fri)).toBe("p1");
  expect(viewSubtitle(inbox, fri)).toBe("");
  expect(canShowCompleted(upcoming)).toBe(false);
  expect(canShowCompleted(filter)).toBe(false);
  expect(canShowCompleted(today)).toBe(true);
  expect(showsProject(today)).toBe(true);
  expect(showsProject(project)).toBe(false);
});

it("empty states match the Dioxus text", () => {
  expect(emptyText(today)).toEqual(["You're all done for today!", "Enjoy the rest of your day."]);
  expect(emptyText(upcoming)).toEqual([
    "Nothing scheduled.",
    "Tasks with dates in the next two weeks show up here.",
  ]);
  expect(emptyText(filter)).toEqual([
    "No tasks match this filter.",
    "Edit it from the sidebar to widen the query.",
  ]);
  expect(emptyText(inbox)).toEqual(["No tasks yet.", "Add one below."]);
});

it("views compare by content", () => {
  expect(sameView({ kind: "project", id: "p", name: "Home" }, project)).toBe(true);
  expect(sameView(project, { kind: "project", id: "q", name: "Home" })).toBe(false);
  expect(sameView(today, inbox)).toBe(false);
});
