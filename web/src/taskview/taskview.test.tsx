// The task view over the mock backend: "Pack for the trip" (Trip, due today,
// labelled errand) has three sub-tasks: "Find the passport" (checked),
// "Pack the bag" (with "Socks" under it) and "Print the tickets".
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useEffect, useState } from "react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import type { Loaded } from "../app/load";
import { useLoaded } from "../app/load";
import type { View } from "../app/view";
import type { Action, Id } from "../bindings";
import { localToday } from "../clock";
import { taskDetail } from "../core";
import { setStreak } from "../ipc/actions";
import { invoke } from "../ipc/invoke";
import { mockHandlers } from "../ipc/mock";
import { loadCoreForTests } from "../test/wasm";
import { TaskNavProvider } from "./nav";
import { useTaskNav } from "./nav-context";
import { TaskView } from "./TaskView";

beforeAll(() => {
  loadCoreForTests();
});

afterEach(() => {
  vi.restoreAllMocks();
});

const INBOX: View = { kind: "inbox" };

/**
 * The stack as the sheet draws it (the view under the top one, inactive, when
 * `under`; then the top one), over the real loader; a change reloads, then
 * prunes the stack against the new tasks (App's job).
 */
// biome-ignore lint/style/useComponentExportOnlyModules: a test harness, never hot-reloaded.
function Stack({
  start,
  under,
  loaded,
  onChanged,
}: {
  start: Id[];
  under: boolean;
  loaded: Loaded;
  onChanged: () => void;
}) {
  const nav = useTaskNav();
  const { open, push, prune } = nav;
  useEffect(() => {
    const [first, ...rest] = start;
    if (first === undefined) return;
    open(first);
    for (const id of rest) push(id);
  }, [start, open, push]);
  useEffect(() => prune(loaded.all), [loaded.all, prune]);
  const shown = nav.stack.slice(under ? -2 : -1);
  return (
    <>
      <output data-testid="stack">{nav.stack.join(" ")}</output>
      <output data-testid="close-asked">{String(nav.closeAsked)}</output>
      <button type="button" data-testid="ask-close" onClick={nav.askClose} />
      {shown.map((id, i) => {
        const detail = taskDetail(id, loaded.all, nav.showCompleted, localToday());
        if (detail === null) return null;
        // `under` with one level shows it inactive on its own.
        const active = i === shown.length - 1 && !(under && shown.length === 1);
        return (
          <TaskView
            key={id}
            detail={detail}
            depth={nav.stack.length - (active ? 0 : 1)}
            active={active}
            loaded={loaded}
            onChanged={onChanged}
          />
        );
      })}
    </>
  );
}

// biome-ignore lint/style/useComponentExportOnlyModules: a test harness, never hot-reloaded.
function Harness({ start, under }: { start: Id[]; under: boolean }) {
  const [reloadKey, setReloadKey] = useState(0);
  const { loaded } = useLoaded(INBOX, false, reloadKey);
  const [first] = useState(start);
  if (loaded === null) return null;
  return (
    <TaskNavProvider all={loaded.all}>
      <Stack start={first} under={under} loaded={loaded} onChanged={() => setReloadKey((k) => k + 1)} />
    </TaskNavProvider>
  );
}

const allTasks = () => invoke("all_tasks", {});
const task = async (id: Id) => (await allTasks()).find((t) => t.id === id);

/** Every action dispatched from here on. */
function spyActions(): Action[] {
  const sent: Action[] = [];
  const real = mockHandlers.dispatch;
  vi.spyOn(mockHandlers, "dispatch").mockImplementation((args) => {
    sent.push(args.cmd.action);
    return real(args);
  });
  return sent;
}

const stack = () => screen.getByTestId("stack").textContent;
const top = () => document.querySelector(".task-view:not(.tv-under)") as HTMLElement;
const titleField = () => top().querySelector(".tv-title textarea") as HTMLTextAreaElement;

async function show(start: Id[], under = false) {
  render(<Harness start={start} under={under} />);
  const last = start.at(-1);
  await waitFor(() => expect(document.querySelector(`.task-view[data-view-id="${last}"]`)).not.toBeNull());
}

it("the header has Close and More, with no name and no Back at the top", async () => {
  await show(["t-trip"]);
  expect(top().querySelector(".tv-head .tv-close.tv-round")?.getAttribute("title")).toBe("Close");
  expect(top().querySelector(".tv-crumb")).toBeNull();
  expect(top().querySelector(".tv-head-actions .tv-more")?.getAttribute("title")).toBe("More");
  expect(top().querySelector(".tv-back")).toBeNull();
  expect(titleField().value).toBe("Pack for the trip");
  expect(top().getAttribute("aria-hidden")).toBe("false");
});

it("a sub-task's view has a round Back in place of Close", async () => {
  await show(["t-trip", "t-bag"]);
  const back = top().querySelector(".tv-head .tv-back.tv-round");
  expect(back?.getAttribute("aria-label")).toBe("Back");
  expect(back?.textContent).toBe("");
  expect(top().querySelector(".tv-close")).toBeNull();
});

it("the sub-task card counts, toggles completed, and strikes the checked row", async () => {
  await show(["t-trip"]);
  const side = () => top().querySelector(".tv-sub-head .tv-sub-side") as HTMLElement;
  expect(side().textContent).toBe("1 of 3 · Show completed");
  expect(top().querySelector(".tv-progress")).toBeNull();
  expect(top().querySelector('.tv-sub-head [data-progress="some"] circle[stroke-dasharray]')).not.toBeNull();
  expect(top().querySelector(".tv-sub-head")?.closest(".tv-subtasks")).toBeNull();
  const rows = () =>
    [...top().querySelectorAll(".tv-sub")].map((r) => r.querySelector(".task-content")?.textContent);
  expect(rows()).toEqual(["Pack the bag", "Print the tickets"]);
  // "Pack the bag" has its own sub-task count and a chevron.
  const bag = top().querySelector(".tv-sub") as HTMLElement;
  expect(bag.querySelector(".tv-sub-body .task-count")?.textContent).toBe("0 of 1");
  expect(bag.querySelector(".tv-sub-body .card-chev")).not.toBeNull();
  fireEvent.click(within(side()).getByText("Show completed"));
  expect(side().textContent).toBe("1 of 3 · Hide completed");
  expect(rows()).toEqual(["Find the passport", "Pack the bag", "Print the tickets"]);
  const done = top().querySelector(".tv-sub.done") as HTMLElement;
  expect(done.querySelector(".task-content")?.textContent).toBe("Find the passport");
  expect(done.querySelector("button.check.done")?.getAttribute("title")).toBe("Mark as not done");
});

it("set fields are rows in order; missing ones are pills", async () => {
  await show(["t-trip"]);
  const rows = [...top().querySelectorAll(".tv-fields > .tv-row")];
  expect(rows.map((r) => r.textContent)).toEqual(["Today", "Trip", "errand"]);
  const date = rows[0]?.querySelector("button") as HTMLButtonElement;
  expect(date.className).toBe("tv-field-btn due-chip today");
  const pills = [...top().querySelectorAll(".tv-pills .tv-pill-slot")].map((p) => p.textContent);
  expect(pills).toEqual(["+ Description", "+ Repeat", "+ Reminders", "+ Deadline", "+ Priority"]);
  expect(top().querySelector(".tv-pills .tv-pill-slot button.tv-pill")).not.toBeNull();
});

const before = (a: Element | null, b: Element | null) =>
  a !== null && b !== null && (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;

it("runs title, description, field card, one row of pills, then sub-tasks", async () => {
  await setStreak("t-trip", false, null);
  await show(["t-trip"]);
  fireEvent.click(within(top()).getByText("+ Description"));
  const desc = top().querySelector(".tv-desc-line");
  const card = top().querySelector(".tv-fields");
  const pills = top().querySelector(".tv-pills");
  const subs = top().querySelector(".tv-subtasks");
  expect(before(top().querySelector(".tv-titleline"), desc)).toBe(true);
  expect(before(desc, card)).toBe(true);
  expect(before(card, pills)).toBe(true);
  expect(before(pills, subs)).toBe(true);
  expect(card?.querySelector(".tv-pills")).toBeNull();
  expect(card?.querySelector(".tv-desc")).toBeNull();
  expect(top().querySelectorAll(".tv-pills")).toHaveLength(1);
});

it("the pills fade on whichever side one runs past", async () => {
  await show(["t-trip"]);
  const row = top().querySelector(".tv-pills") as HTMLElement;
  const wrap = () => row.parentElement?.className;
  Object.defineProperties(row, {
    clientWidth: { configurable: true, value: 300 },
    scrollWidth: { configurable: true, value: 700 },
  });
  fireEvent.scroll(row);
  expect(wrap()).toContain("pills-run-on");
  expect(wrap()).not.toContain("pills-run-back");
  row.scrollLeft = 400;
  fireEvent.scroll(row);
  expect(wrap()).not.toContain("pills-run-on");
  expect(wrap()).toContain("pills-run-back");
});

it("the pills never wrap, at any width", () => {
  const css = readFileSync(resolve(process.cwd(), "src/styles/app.css"), "utf8");
  expect(css).not.toMatch(/\.tv-pills\s*\{[^}]*flex-wrap:\s*wrap/);
});

it("a repeat gets its own row, under the date", async () => {
  await show(["t-trip"]);
  fireEvent.click(within(top()).getByText("+ Repeat"));
  fireEvent.click(within(top()).getByText("Every day"));
  await waitFor(() => {
    const rows = [...top().querySelectorAll(".tv-fields > .tv-row")].map((r) => r.textContent);
    expect(rows.slice(0, 2)).toEqual(["Today", "Every day"]);
  });
});

it("a tracked streak's row shows current and best, and opens the streak panel", async () => {
  await setStreak("t-trip", true, null);
  await show(["t-trip"]);
  const row = top().querySelector(".tv-fields > .tv-row button.tv-streak") as HTMLElement;
  // keel-core's wording; this task doesn't repeat, so it's the generic one.
  expect(row.textContent).toBe("Streak of 00");
  expect(row.querySelector('[title="Best"] svg')).not.toBeNull();
  expect(row.querySelector('[title="Goal"]')).toBeNull();
  expect(row.querySelector("svg")).not.toBeNull();
  await setStreak("t-trip", false, null);
});

it("the streak row opens the streak in a popover, closed by a tap outside", async () => {
  await setStreak("t-trip", true, null);
  await show(["t-trip"]);
  expect(top().querySelector(".streak-panel")).toBeNull();
  fireEvent.click(top().querySelector(".tv-streak") as HTMLElement);
  const panel = top().querySelector(".tv-row .popover .streak-panel");
  expect(panel?.textContent).toContain("Stop tracking");
  fireEvent.click(top().querySelector(".pop-backdrop") as HTMLElement);
  expect(top().querySelector(".streak-panel")).toBeNull();
  await setStreak("t-trip", false, null);
});

it("a goal typed in the popover is saved when it closes, without the field being left", async () => {
  await setStreak("t-trip", true, null);
  await show(["t-trip"]);
  fireEvent.click(top().querySelector(".tv-streak") as HTMLElement);
  const goal = top().querySelector(".streak-panel input") as HTMLInputElement;
  fireEvent.change(goal, { target: { value: "30" } });
  fireEvent.click(top().querySelector(".pop-backdrop") as HTMLElement);
  await waitFor(async () => {
    const saved = (await invoke("all_tasks", {})).find((t) => t.id === "t-trip");
    expect(saved?.streak_goal).toBe(30);
  });
  await setStreak("t-trip", false, null);
});

it("the streak row shows a goal that's set", async () => {
  await setStreak("t-trip", true, 30);
  await show(["t-trip"]);
  expect(top().querySelector('.tv-streak [title="Goal"]')?.textContent).toBe("30");
  expect(top().querySelector('.tv-streak [title="Goal"] svg')).not.toBeNull();
  await setStreak("t-trip", false, null);
});

it("on touch, a tracked streak opens on the keyboard with its goal focused inside the tap", async () => {
  vi.stubGlobal("matchMedia", (q: string) => ({ matches: q.includes("coarse") }));
  await setStreak("t-trip", true, null);
  await show(["t-trip"]);
  fireEvent.click(top().querySelector(".tv-streak") as HTMLElement);
  // On the page, not in the sheet, whose expanding would carry it down first.
  const card = document.querySelector("body > .streak-pop.keyboard-card") as HTMLElement;
  expect(document.activeElement).toBe(card.querySelector("input"));
  fireEvent.click(top().querySelector(".pop-backdrop") as HTMLElement);
  expect(document.querySelector(".streak-pop")).toBeNull();
  await setStreak("t-trip", false, null);
  vi.unstubAllGlobals();
});

it("no streak row without a tracked streak", async () => {
  await setStreak("t-trip", false, null);
  await show(["t-trip"]);
  expect(top().querySelector(".tv-streak")).toBeNull();
});

it("an Inbox task has no project row, and a + Project pill", async () => {
  await show(["t-inbox"]);
  const rows = [...top().querySelectorAll(".tv-fields > .tv-row")].map((r) => r.textContent);
  expect(rows).not.toContain("Inbox");
  const pills = [...top().querySelectorAll(".tv-pills .tv-pill-slot")].map((p) => p.textContent);
  expect(pills).toContain("+ Project");
});

it("without sub-tasks there's no count, only Add sub-task", async () => {
  await show(["t-inbox"]);
  expect(top().querySelector(".tv-sub-head")?.textContent).toBe("Sub-tasks");
  expect(top().querySelector(".tv-subtasks .tv-add-sub")?.textContent).toBe("Add sub-task");
  expect(top().querySelector(".tv-sub")).toBeNull();
});

it("picking Priority 1 from its pill saves it and it becomes a row", async () => {
  await show(["t-trip"]);
  fireEvent.click(within(top()).getByText("+ Priority"));
  fireEvent.click(within(top()).getByText("Priority 1"));
  await waitFor(async () => expect((await task("t-trip"))?.priority).toBe("P1"));
  await waitFor(() =>
    expect([...top().querySelectorAll(".tv-fields > .tv-row")].map((r) => r.textContent)).toContain(
      "Priority 1",
    ),
  );
  expect(within(top()).queryByText("+ Priority")).toBeNull();
});

it("+ Description opens a focused field; typing and leaving it saves", async () => {
  await show(["t-trip"]);
  fireEvent.click(within(top()).getByText("+ Description"));
  const field = top().querySelector(".tv-desc-line .tv-desc textarea") as HTMLTextAreaElement;
  expect(field).not.toBeNull();
  expect(document.activeElement).toBe(field);
  expect(field.getAttribute("placeholder")).toBe("Description");
  fireEvent.change(field, { target: { value: "Passport,\ncharger" } });
  fireEvent.blur(field);
  await waitFor(async () => expect((await task("t-trip"))?.description).toBe("Passport,\ncharger"));
});

it("a typed title saves on Back, once, and the stack pops", async () => {
  await show(["t-trip", "t-bag"]);
  const sent = spyActions();
  fireEvent.change(titleField(), { target: { value: "Pack bag" } });
  fireEvent.click(top().querySelector(".tv-back") as HTMLElement);
  expect(stack()).toBe("t-trip");
  await waitFor(async () => expect((await task("t-bag"))?.content).toBe("Pack bag"));
  expect(sent).toEqual([expect.objectContaining({ type: "task_update", id: "t-bag", content: "Pack bag" })]);
});

it("a retyped title applies what it says and loses those words", async () => {
  await show(["t-trip"]);
  fireEvent.change(titleField(), { target: { value: "Pack for the trip p1 tomorrow" } });
  expect(top().querySelector(".tv-grow-marks mark")).not.toBeNull();
  fireEvent.blur(titleField());
  await waitFor(async () => expect((await task("t-trip"))?.content).toBe("Pack for the trip"));
  const saved = await task("t-trip");
  expect(saved?.priority).toBe("P1");
  expect(saved?.due?.date).not.toBe(undefined);
});

it("backspacing a phrase to plain keeps it as text and leaves the fields alone", async () => {
  await show(["t-trip"]);
  const sent = spyActions();
  const field = titleField();
  fireEvent.change(field, { target: { value: "Pack for the trip p1" } });
  field.setSelectionRange(field.value.length, field.value.length);
  fireEvent.keyDown(field, { key: "Backspace" });
  expect(top().querySelector(".tv-grow-marks mark")).toBeNull();
  fireEvent.blur(field);
  await waitFor(async () => expect((await task("t-trip"))?.content).toBe("Pack for the trip p1"));
  expect(sent).toEqual([
    expect.objectContaining({ type: "task_update", id: "t-trip", content: "Pack for the trip p1" }),
  ]);
  expect((sent[0] as { priority?: unknown }).priority ?? null).toBeNull();
});

it("an emptied title goes back to the saved one and sends nothing", async () => {
  await show(["t-trip"]);
  const sent = spyActions();
  fireEvent.change(titleField(), { target: { value: "   " } });
  fireEvent.blur(titleField());
  expect(titleField().value).toBe("Pack for the trip");
  await act(async () => {});
  expect(sent).toEqual([]);
});

it("Enter in the title leaves it, which saves", async () => {
  await show(["t-trip"]);
  titleField().focus();
  fireEvent.change(titleField(), { target: { value: "Pack for Rome" } });
  fireEvent.keyDown(titleField(), { key: "Enter" });
  expect(document.activeElement).not.toBe(titleField());
  await waitFor(async () => expect((await task("t-trip"))?.content).toBe("Pack for Rome"));
});

it("a double tap on the title circle completes once and goes back", async () => {
  await show(["t-trip", "t-bag"]);
  const sent = spyActions();
  const circle = top().querySelector(".tv-titleline button.check") as HTMLButtonElement;
  expect(circle.getAttribute("title")).toBe("Complete task");
  fireEvent.click(circle);
  fireEvent.click(circle);
  await waitFor(() => expect(stack()).toBe("t-trip"));
  expect(sent.filter((a) => a.type === "task_complete")).toHaveLength(1);
  expect((await task("t-bag"))?.checked).toBe(true);
});

it("a repeating sub-task done for today shows done in its own view, and its circle undoes it", async () => {
  const next = new Date(`${localToday()}T12:00:00`);
  next.setDate(next.getDate() + 1);
  const tomorrow = `${next.getFullYear()}-${String(next.getMonth() + 1).padStart(2, "0")}-${String(next.getDate()).padStart(2, "0")}`;
  const real = mockHandlers.all_tasks;
  vi.spyOn(mockHandlers, "all_tasks").mockImplementation(async (args) =>
    (await real(args)).map((t) =>
      t.id === "t-bag"
        ? {
            ...t,
            due: {
              date: tomorrow,
              timezone: null,
              string: "every day",
              lang: "en",
              is_recurring: true,
              recurrence: "Daily",
            },
            last_completion: {
              moved_to: tomorrow,
              date: localToday(),
              streak_current: 0,
              streak_best: 0,
              streak_last_period: null,
            },
          }
        : t,
    ),
  );
  await show(["t-trip", "t-bag"]);
  const sent = spyActions();
  const circle = top().querySelector(".tv-titleline button.check.done") as HTMLButtonElement;
  expect(circle.title).toBe("Mark as not done");
  fireEvent.click(circle);
  await waitFor(() => expect(sent).toEqual([{ type: "task_uncomplete", id: "t-bag" }]));
});

it("a sub-task's circle completes it and the view stays", async () => {
  await show(["t-trip"]);
  const row = within(top()).getByText("Print the tickets").closest(".tv-sub") as HTMLElement;
  fireEvent.click(within(row).getByTitle("Complete sub-task"));
  await waitFor(async () => expect((await task("t-tickets"))?.checked).toBe(true));
  await waitFor(() => expect(within(top()).queryByText("Print the tickets")).toBeNull());
  expect(stack()).toBe("t-trip");
  expect(top().querySelector(".tv-sub-side")?.textContent).toBe("2 of 3 · Show completed");
});

it("a checked sub-task's circle reopens it", async () => {
  await show(["t-trip"]);
  fireEvent.click(within(top()).getByText("Show completed"));
  const row = within(top()).getByText("Find the passport").closest(".tv-sub") as HTMLElement;
  fireEvent.click(within(row).getByTitle("Mark as not done"));
  await waitFor(async () => expect((await task("t-passport"))?.checked).toBe(false));
});

it("tapping a sub-task's body opens its view on top", async () => {
  await show(["t-trip"]);
  const body = within(top()).getByText("Pack the bag").closest(".tv-sub-body") as HTMLElement;
  expect(body.getAttribute("title")).toBe("Open task");
  fireEvent.click(body);
  expect(stack()).toBe("t-trip t-bag");
  await waitFor(() => expect(titleField().value).toBe("Pack the bag"));
});

it("Delete task in ••• deletes, and the view closes after the reload", async () => {
  await show(["t-trip"]);
  fireEvent.click(top().querySelector(".tv-more") as HTMLElement);
  const items = [...top().querySelectorAll(".popover.menu .menu-item")];
  // Not repeating: no Streak.
  expect(items.map((i) => i.textContent)).toEqual(["Delete task"]);
  expect(items[0]?.className).toBe("menu-item danger");
  fireEvent.click(items[0] as HTMLElement);
  await waitFor(() => expect(stack()).toBe(""));
  expect((await allTasks()).some((t) => t.id === "t-trip")).toBe(false);
});

it("Add sub-task opens the editor; Enter adds under this task and it stays open", async () => {
  await show(["t-trip"]);
  fireEvent.click(top().querySelector(".tv-add-sub") as HTMLElement);
  const title = top().querySelector(".tv-add-editor .editor textarea") as HTMLTextAreaElement;
  expect(document.activeElement).toBe(title);
  expect(within(top()).getByText("Add sub-task", { selector: "button" })).toBeTruthy();
  fireEvent.change(title, { target: { value: "Buy adapters" } });
  await act(async () => {
    fireEvent.keyDown(title, { key: "Enter" });
  });
  await waitFor(async () => {
    const added = (await allTasks()).find((t) => t.content === "Buy adapters");
    expect(added?.parent_id).toBe("t-trip");
    expect(added?.project_id).toBe("p-trip");
  });
  await waitFor(() => expect(within(top()).getByText("Buy adapters")).toBeTruthy());
  expect(top().querySelector(".tv-add-editor .editor")).not.toBeNull();
  expect((top().querySelector(".tv-add-editor .editor textarea") as HTMLTextAreaElement).value).toBe("");
});

it("the view under the top one has no header and ignores a close request", async () => {
  await show(["t-trip", "t-bag"], true);
  const under = document.querySelector('.task-view[data-view-id="t-trip"]') as HTMLElement;
  expect(under.className).toBe("task-view tv-under");
  expect(under.getAttribute("aria-hidden")).toBe("true");
  expect(under.querySelector(".tv-head-space")).not.toBeNull();
  expect(under.querySelector(".tv-head")).toBeNull();
  expect(top().querySelector(".tv-head-space")).toBeNull();
});

it("an inactive view leaves closeAsked for the active one", async () => {
  // Only the inactive view is mounted (`under` with one level).
  await show(["t-trip"], true);
  expect(document.querySelector(".task-view.tv-under")).not.toBeNull();
  fireEvent.click(screen.getByTestId("ask-close"));
  await act(async () => {});
  expect(screen.getByTestId("close-asked").textContent).toBe("true");
  expect(stack()).toBe("t-trip");
});

it("the active view closes when asked (scrim or drag), saving its title first", async () => {
  await show(["t-trip"]);
  fireEvent.change(titleField(), { target: { value: "Pack for Rome" } });
  fireEvent.click(screen.getByTestId("ask-close"));
  await waitFor(() => expect(stack()).toBe(""));
  expect(screen.getByTestId("close-asked").textContent).toBe("false");
  await waitFor(async () => expect((await task("t-trip"))?.content).toBe("Pack for Rome"));
});

it("Close closes at once outside the phone sheet", async () => {
  await show(["t-trip"]);
  fireEvent.click(top().querySelector(".tv-close") as HTMLElement);
  expect(stack()).toBe("");
});

/** "Pack for the trip" repeating every day from today, with a streak of 3. */
function streakTrip() {
  const real = mockHandlers.all_tasks;
  vi.spyOn(mockHandlers, "all_tasks").mockImplementation(async (args) =>
    (await real(args)).map((t) =>
      t.id === "t-trip"
        ? {
            ...t,
            due: {
              date: localToday(),
              timezone: null,
              string: "every day",
              lang: "en",
              is_recurring: true,
              recurrence: "Daily",
            },
            streak_enabled: true,
            streak_current: 3,
            streak_best: 5,
          }
        : t,
    ),
  );
}

const moveToTomorrow = () => {
  fireEvent.click(within(top()).getByText("Today"));
  fireEvent.click(within(document.querySelector(".scheduler") as HTMLElement).getByText("Tomorrow"));
};

it("moving a task with a streak asks first, and Cancel leaves it", async () => {
  streakTrip();
  await show(["t-trip"]);
  const sent = spyActions();
  moveToTomorrow();
  const prompt = document.querySelector(".move-prompt") as HTMLElement;
  expect(prompt.textContent).toContain("streak of 3");
  fireEvent.click(within(prompt).getByText("Cancel"));
  expect(document.querySelector(".move-prompt")).toBeNull();
  await new Promise((r) => setTimeout(r, 50));
  expect(sent.filter((a) => a.type === "task_update")).toHaveLength(0);
  expect(within(top()).getByText("Today")).not.toBeNull();
});

it("Move saves the new date", async () => {
  streakTrip();
  await show(["t-trip"]);
  const sent = spyActions();
  moveToTomorrow();
  fireEvent.click(within(document.querySelector(".move-prompt") as HTMLElement).getByText("Move"));
  await waitFor(() => expect(sent.filter((a) => a.type === "task_update")).toHaveLength(1));
  expect(document.querySelector(".move-prompt")).toBeNull();
});

it("a move with no streak going saves without asking", async () => {
  await show(["t-trip"]);
  const sent = spyActions();
  moveToTomorrow();
  expect(document.querySelector(".move-prompt")).toBeNull();
  await waitFor(() => expect(sent.filter((a) => a.type === "task_update")).toHaveLength(1));
});

it("a date typed into the title of a task with a streak asks first; Cancel puts the title back", async () => {
  streakTrip();
  await show(["t-trip"]);
  const sent = spyActions();
  fireEvent.change(titleField(), { target: { value: "Pack for the trip tomorrow" } });
  fireEvent.blur(titleField());
  const prompt = document.querySelector(".move-prompt") as HTMLElement;
  expect(prompt.textContent).toContain("streak of 3");
  fireEvent.click(within(prompt).getByText("Cancel"));
  expect(document.querySelector(".move-prompt")).toBeNull();
  expect(titleField().value).toBe("Pack for the trip");
  await new Promise((r) => setTimeout(r, 50));
  expect(sent.filter((a) => a.type === "task_update")).toHaveLength(0);
});

it("a typed move asks before the view closes, then closes once answered", async () => {
  streakTrip();
  await show(["t-trip"]);
  const sent = spyActions();
  fireEvent.change(titleField(), { target: { value: "Pack for the trip tomorrow" } });
  fireEvent.click(top().querySelector(".tv-close") as HTMLElement);
  expect(stack()).toBe("t-trip");
  fireEvent.click(within(document.querySelector(".move-prompt") as HTMLElement).getByText("Move"));
  expect(stack()).toBe("");
  await waitFor(() => expect(sent.filter((a) => a.type === "task_update")).toHaveLength(1));
});
