// The task editor, with the clock at Friday 2026-10-30 noon.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { EditorValues } from "../bindings";
import { blankValues, valuesFromTask } from "../core";
import * as fx from "../ipc/fixtures";
import { mockPageScrollHeld } from "../ipc/mock";
import { CheckSmallIcon } from "../shell/icons";
import { loadCoreForTests } from "../test/wasm";
import { TaskEditor } from "./TaskEditor";

beforeAll(() => {
  loadCoreForTests();
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(2026, 9, 30, 12));
});

afterAll(() => {
  vi.useRealTimers();
});

let onSubmit = vi.fn<(v: EditorValues) => void>();
let onCancel = vi.fn<() => void>();

beforeEach(() => {
  onSubmit = vi.fn();
  onCancel = vi.fn();
});

function adding() {
  render(
    <TaskEditor
      initial={blankValues("p-inbox", null, "", [])}
      projects={fx.projects}
      sections={fx.sections}
      labels={fx.labels}
      keepLabel={null}
      existing={null}
      submitText="Add task"
      quickAdd
      keepOpen
      onSubmit={onSubmit}
      onCancel={onCancel}
      onChanged={() => {}}
    />,
  );
  return screen.getByPlaceholderText(
    "Task name — try “Call mom tomorrow 5pm p1 #Home @phone”",
  ) as HTMLTextAreaElement;
}

const type = (title: HTMLTextAreaElement, text: string) =>
  fireEvent.change(title, { target: { value: text } });
const marks = () =>
  [...document.querySelectorAll(".title-mirror mark")].map((m) => `${m.className}:${m.textContent}`);
const backspaceAt = (title: HTMLTextAreaElement, at: number) => {
  title.setSelectionRange(at, at);
  return fireEvent.keyDown(title, { key: "Backspace" });
};

const PILLS = [
  "Date",
  "Deadline: the date it must be done by",
  "Priority",
  "Project / section",
  "Labels",
  "Reminders",
  "Repeat",
];

it("with a mouse too, clicking the title or description closes a picker", () => {
  const title = adding();
  fireEvent.click(screen.getByTitle("Priority"));
  fireEvent.pointerUp(title);
  expect(document.querySelector(".popover")).toBeNull();
  fireEvent.click(screen.getByTitle("Priority"));
  fireEvent.pointerUp(screen.getByPlaceholderText("Description"));
  expect(document.querySelector(".popover")).toBeNull();
  cleanup();
});

describe("the description", () => {
  const desc = () => screen.getByPlaceholderText("Description") as HTMLTextAreaElement;

  it("grows with its text, as the title does", () => {
    adding();
    expect(desc().tagName).toBe("TEXTAREA");
    fireEvent.change(desc(), { target: { value: "Passport,\ncharger" } });
    expect(document.querySelector(".desc-mirror")?.textContent).toContain("Passport,\ncharger");
    cleanup();
  });

  it("takes Enter as a new line, and saves it", () => {
    const title = adding();
    type(title, "Pack");
    fireEvent.change(desc(), { target: { value: "Passport,\ncharger" } });
    expect(fireEvent.keyDown(desc(), { key: "Enter" })).toBe(true);
    expect(onSubmit).not.toHaveBeenCalled();
    fireEvent.submit(document.querySelector("form.editor") as HTMLFormElement);
    expect(onSubmit.mock.calls[0]?.[0].description).toBe("Passport,\ncharger");
    cleanup();
  });

  it("submits on Cmd or Ctrl+Enter", () => {
    const title = adding();
    type(title, "Pack");
    expect(fireEvent.keyDown(desc(), { key: "Enter", metaKey: true })).toBe(false);
    expect(onSubmit).toHaveBeenCalledTimes(1);
    type(title, "Pack again");
    fireEvent.keyDown(desc(), { key: "Enter", ctrlKey: true });
    expect(onSubmit).toHaveBeenCalledTimes(2);
    cleanup();
  });
});

describe("a card too long for the screen", () => {
  beforeEach(() => {
    vi.stubGlobal("matchMedia", (q: string) => ({ matches: q.includes("coarse") }));
    vi.stubGlobal("visualViewport", Object.assign(new EventTarget(), { offsetTop: 0, height: 478 }));
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    cleanup();
  });

  const form = () => document.querySelector<HTMLFormElement>("form.editor");
  const text = () => document.querySelector<HTMLElement>(".editor-text");

  it("scrolls the title and description together, and not the pills or save button", () => {
    const title = adding();
    type(title, "Call mom tomorrow");
    expect(text()?.contains(title)).toBe(true);
    expect(text()?.contains(screen.getByPlaceholderText("Description"))).toBe(true);
    expect(text()?.querySelector(".quick-preview")).not.toBeNull();
    expect(text()?.querySelector(".chips")).toBeNull();
    expect(text()?.contains(screen.getByRole("button", { name: "Add task" }))).toBe(false);
  });

  it("knows the room it has above the keyboard", () => {
    adding();
    expect(form()?.style.getPropertyValue("--card-room")).toBe("478px");
  });

  it("makes way for an open picker", () => {
    adding();
    expect(form()?.className).not.toContain("picking");
    fireEvent.click(screen.getByTitle("Priority"));
    expect(form()?.className).toContain("picking");
  });
});

describe("on touch", () => {
  beforeEach(() => {
    vi.stubGlobal("matchMedia", (q: string) => ({ matches: q.includes("coarse") }));
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    cleanup();
  });

  const form = () => document.querySelector<HTMLFormElement>("form.editor");

  it("sits on the keyboard, outside anything that would move it", () => {
    const { container } = render(<div data-testid="host" />);
    adding();
    expect(form()?.className).toContain("on-keyboard");
    expect(form()?.parentElement).toBe(document.body);
    expect(container.querySelector("form")).toBeNull();
  });

  it("scrolls its pills in the approved order, with a fixed save button and no Cancel", () => {
    adding();
    const pills = [...document.querySelectorAll<HTMLElement>(".chips .chip-btn")].map((b) => b.title);
    expect(pills).toEqual(PILLS);
    const save = screen.getByRole("button", { name: "Add task" });
    expect(save.closest(".chips")).toBeNull();
    expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();
  });

  it("a tap outside cancels; with a picker open it closes the picker first", () => {
    adding();
    fireEvent.click(screen.getByTitle("Priority"));
    expect(document.querySelector(".popover")).not.toBeNull();
    fireEvent.click(document.querySelector(".pop-backdrop") as Element);
    expect(document.querySelector(".popover")).toBeNull();
    expect(onCancel).not.toHaveBeenCalled();
    fireEvent.click(document.querySelector(".editor-scrim") as Element);
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("keeps the title focused when a pill, a picker or the area outside is pressed", () => {
    adding();
    expect(fireEvent.mouseDown(screen.getByTitle("Priority"))).toBe(false);
    fireEvent.click(screen.getByTitle("Priority"));
    expect(fireEvent.mouseDown(document.querySelector(".pop-backdrop") as Element)).toBe(false);
    expect(fireEvent.mouseDown(screen.getByText("Priority 1"))).toBe(false);
  });

  it("lets a text field take focus", () => {
    adding();
    expect(fireEvent.mouseDown(screen.getByPlaceholderText("Description"))).toBe(true);
  });

  it("dims the page from outside the card, which would dim the card's own color from inside", () => {
    adding();
    expect(document.querySelector(".editor-scrim")?.closest("form")).toBeNull();
    fireEvent.click(screen.getByTitle("Priority"));
    expect(document.querySelector(".pop-backdrop")?.closest("form")).toBeNull();
  });

  it("keeps one dim layer as pickers open and close, so it doesn't flash", () => {
    adding();
    const dim = document.querySelector(".editor-scrim");
    fireEvent.click(screen.getByTitle("Priority"));
    expect(document.querySelector(".pop-backdrop")).toBe(dim);
    expect(document.querySelector(".editor-scrim")).toBe(dim);
    fireEvent.click(dim as Element);
    expect(document.querySelector(".popover")).toBeNull();
    expect(document.querySelector(".editor-scrim")).toBe(dim);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("fits the card in the room above the keyboard, so long text scrolls and it stays on screen", () => {
    const css = readFileSync(resolve(process.cwd(), "src/styles/app.css"), "utf8");
    expect(css).toMatch(/\.editor\.on-keyboard \{[^}]*max-height: calc\(var\(--card-room/);
    expect(css).toMatch(/\.on-keyboard \.editor-text \{[^}]*overflow-y: auto/);
  });

  it("floats a small gap above the keyboard", () => {
    vi.stubGlobal("visualViewport", Object.assign(new EventTarget(), { offsetTop: 0, height: 546 }));
    const client = vi.spyOn(document.documentElement, "clientHeight", "get").mockReturnValue(874);
    adding();
    expect(document.querySelector<HTMLElement>("form.editor")?.style.bottom).toBe("336px");
    client.mockRestore();
  });

  it("fades the pills only while one runs under the save button", () => {
    adding();
    const row = document.querySelector(".chips") as HTMLElement;
    const actions = () => document.querySelector(".editor-actions")?.className;
    Object.defineProperties(row, {
      clientWidth: { configurable: true, value: 300 },
      scrollWidth: { configurable: true, value: 700 },
    });
    fireEvent.scroll(row);
    expect(actions()).toContain("pills-run-on");
    expect(actions()).not.toContain("pills-run-back");
    row.scrollLeft = 100;
    fireEvent.scroll(row);
    expect(actions()).toContain("pills-run-on");
    expect(actions()).toContain("pills-run-back");
    row.scrollLeft = 400;
    fireEvent.scroll(row);
    expect(actions()).not.toContain("pills-run-on");
    expect(actions()).toContain("pills-run-back");
  });

  it("the open Priority pill keeps its color", () => {
    adding();
    const pill = () => screen.getByTitle("Priority");
    fireEvent.click(pill());
    expect(pill().className).not.toContain("pill-tinted");
    fireEvent.click(screen.getByText("Priority 1"));
    fireEvent.click(pill());
    expect(pill().className).toContain("chip-open");
    expect(pill().className).toContain("pill-p1");
    expect(pill().className).toContain("pill-tinted");
  });

  it("the Priority pill shows a flag, in its color once one is set", () => {
    adding();
    const pill = () => screen.getByTitle("Priority");
    expect(pill().querySelector(".chip-icon.pri-4 svg")).not.toBeNull();
    fireEvent.click(pill());
    fireEvent.click(screen.getByText("Priority 2"));
    expect(pill().querySelector(".chip-icon.pri-2 svg")).not.toBeNull();
    expect(pill().className).toContain("pill-p2");
  });

  it("the Project pill shows the icon of where the task goes", () => {
    adding();
    const pill = () => screen.getByTitle("Project / section");
    expect(pill().querySelector(".chip-icon.loc-inbox svg")).not.toBeNull();
    fireEvent.click(pill());
    fireEvent.click(screen.getByText("Trip"));
    expect(pill().querySelector(".chip-icon.loc-project svg")).not.toBeNull();
  });

  it("the Labels pill shows a tag", () => {
    adding();
    expect(screen.getByTitle("Labels").querySelector(".chip-icon svg")).not.toBeNull();
  });

  describe("making room for a calendar", () => {
    const title = () => document.querySelector(".title-input") as HTMLTextAreaElement;
    const focused = () => document.activeElement === title();

    it("the quick choices leave the keyboard up; the calendar lowers it", () => {
      adding();
      title().focus();
      fireEvent.click(screen.getByTitle("Date"));
      expect(focused()).toBe(true);
      fireEvent.click(screen.getByText("Pick a date and time"));
      expect(focused()).toBe(false);
    });

    const openDeadline = () => fireEvent.click(screen.getByTitle("Deadline: the date it must be done by"));
    const openCalendar = () => {
      fireEvent.click(screen.getByTitle("Date"));
      fireEvent.click(screen.getByText("Pick a date and time"));
    };
    const day = (n: string) => screen.getByText(n, { selector: ".cal-day:not(.other)" });

    it("the calendar's X, month and check share one row", () => {
      adding();
      openDeadline();
      const row = document.querySelector(".sched-head") as HTMLElement;
      expect(within(row).getByRole("button", { name: "Cancel" })).toBeTruthy();
      expect(row.querySelector(".cal-title")?.textContent).toBe("October 2026");
      expect(within(row).getByRole("button", { name: "Done" })).toBeTruthy();
    });

    it("the calendar's Done is the plain check, not the check-circle", () => {
      adding();
      openDeadline();
      const done = screen.getByRole("button", { name: "Done" }).querySelector("svg")?.innerHTML;
      const { container, unmount } = render(<CheckSmallIcon />);
      const plain = container.querySelector("svg")?.innerHTML;
      unmount();
      expect(done).toBe(plain);
    });

    it("Deadline leaves it up; a day selects and stays until the check", () => {
      adding();
      title().focus();
      openDeadline();
      expect(focused()).toBe(true);
      fireEvent.click(day("12"));
      expect(document.querySelector(".cal-day.selected")?.textContent).toBe("12");
      expect(focused()).toBe(true);
      fireEvent.click(screen.getByRole("button", { name: "Done" }));
      expect(document.querySelector(".popover")).toBeNull();
      expect(focused()).toBe(true);
      expect(screen.getByTitle("Deadline: the date it must be done by").textContent).toContain("12");
    });

    it("the deadline has no No deadline row, and tapping its day clears it", () => {
      adding();
      openDeadline();
      expect(screen.queryByText("No deadline")).toBeNull();
      fireEvent.click(day("12"));
      expect(document.querySelector(".cal-day.selected")?.textContent).toBe("12");
      fireEvent.click(day("12"));
      expect(document.querySelector(".cal-day.selected")).toBeNull();
      fireEvent.click(screen.getByRole("button", { name: "Done" }));
      expect(screen.getByTitle("Deadline: the date it must be done by").textContent).not.toContain("12");
    });

    it("only the deadline's calendar is compact, to fit above the keyboard", () => {
      adding();
      openDeadline();
      expect(document.querySelector(".scheduler")?.className).toContain("scheduler-compact");
      fireEvent.click(screen.getByRole("button", { name: "Done" }));
      openCalendar();
      expect(document.querySelector(".scheduler")?.className).not.toContain("scheduler-compact");
    });

    it("the X undoes the deadline and leaves the keyboard up", () => {
      adding();
      title().focus();
      openDeadline();
      fireEvent.click(day("12"));
      fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
      expect(document.querySelector(".popover")).toBeNull();
      expect(focused()).toBe(true);
      expect(screen.getByTitle("Deadline: the date it must be done by").textContent).not.toContain("12");
    });

    it("the X undoes the date, time and repeat chosen in the date card", () => {
      adding();
      title().focus();
      fireEvent.click(screen.getByTitle("Date"));
      fireEvent.click(screen.getByText("Pick a date and time"));
      fireEvent.click(day("12"));
      fireEvent.click(
        within(document.querySelector(".scheduler") as HTMLElement).getByRole("button", { name: /^Repeat/ }),
      );
      fireEvent.click(screen.getByText("Every day"));
      fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
      expect(focused()).toBe(true);
      expect(screen.getByTitle("Date").textContent).not.toContain("12");
      expect(screen.getByTitle("Repeat").textContent).not.toContain("Every day");
    });

    it("the quick choices need no check or X, with the keyboard still up", () => {
      adding();
      fireEvent.click(screen.getByTitle("Date"));
      expect(screen.queryByRole("button", { name: "Done" })).toBeNull();
    });

    const desc = () => screen.getByPlaceholderText("Description") as HTMLTextAreaElement;

    it("lowers the keyboard from the description too, and goes back to it", () => {
      adding();
      desc().focus();
      openCalendar();
      expect(document.activeElement).not.toBe(desc());
      fireEvent.click(screen.getByRole("button", { name: "Done" }));
      expect(document.activeElement).toBe(desc());
    });

    it("closing a picker from its own field goes back to the title", () => {
      adding();
      fireEvent.click(screen.getByTitle("Labels"));
      const find = screen.getByPlaceholderText("Find or add a label") as HTMLInputElement;
      find.focus();
      fireEvent.click(document.querySelector(".pop-backdrop") as Element);
      expect(focused()).toBe(true);
    });

    it("another pill or a tap outside brings it back", () => {
      adding();
      title().focus();
      openCalendar();
      expect(focused()).toBe(false);
      fireEvent.click(screen.getByTitle("Priority"));
      expect(focused()).toBe(true);
      fireEvent.click(screen.getByTitle("Priority"));
      openCalendar();
      expect(focused()).toBe(false);
      fireEvent.click(document.querySelector(".pop-backdrop") as Element);
      expect(focused()).toBe(true);
    });

    it("going from the date's calendar to the deadline brings it back", () => {
      adding();
      title().focus();
      openCalendar();
      openDeadline();
      expect(document.querySelector(".cal-grid")).not.toBeNull();
      expect(focused()).toBe(true);
    });

    // iOS sends no click for a tap in the field that already has the cursor.
    it("tapping the focused title or description closes a picker, with no click", () => {
      adding();
      title().focus();
      fireEvent.click(screen.getByTitle("Priority"));
      fireEvent.pointerUp(title());
      expect(document.querySelector(".popover")).toBeNull();
      expect(focused()).toBe(true);

      desc().focus();
      fireEvent.click(screen.getByTitle("Priority"));
      fireEvent.pointerUp(desc());
      expect(document.querySelector(".popover")).toBeNull();
      expect(document.activeElement).toBe(desc());
    });

    it("typing in the title or description closes a picker and keeps what it chose", () => {
      adding();
      title().focus();
      openDeadline();
      fireEvent.click(day("12"));
      fireEvent.change(title(), { target: { value: "Typed" } });
      expect(document.querySelector(".popover")).toBeNull();
      expect(title().value).toBe("Typed");
      expect(screen.getByTitle("Deadline: the date it must be done by").textContent).toContain("12");

      fireEvent.click(screen.getByTitle("Priority"));
      fireEvent.change(desc(), { target: { value: "Notes" } });
      expect(document.querySelector(".popover")).toBeNull();
      expect(desc().value).toBe("Notes");
    });

    it("typing in a picker's own field leaves the picker open", () => {
      adding();
      fireEvent.click(screen.getByTitle("Labels"));
      fireEvent.change(screen.getByPlaceholderText("Find or add a label"), { target: { value: "er" } });
      expect(document.querySelector(".popover")).not.toBeNull();
    });

    it("tapping the description from the date's calendar stays there and keeps the day", () => {
      adding();
      title().focus();
      openCalendar();
      fireEvent.click(day("12"));
      desc().focus();
      fireEvent.pointerUp(desc());
      expect(document.querySelector(".popover")).toBeNull();
      expect(document.activeElement).toBe(desc());
      expect(screen.getByTitle("Date").textContent).toContain("12");
      // The next picker has no keyboard to give back.
      fireEvent.click(screen.getByTitle("Priority"));
      fireEvent.click(document.querySelector(".pop-backdrop") as Element);
      expect(document.activeElement).toBe(desc());
    });

    it("the title stays above a picker's backdrop, so one click closes it and types", () => {
      const css = readFileSync(resolve(process.cwd(), "src/styles/app.css"), "utf8");
      expect(css).toMatch(/\.editor \.title-wrap, \.editor \.desc \{ position: relative; z-index: 20; \}/);
    });
  });

  it("switches pills in one tap", () => {
    adding();
    fireEvent.click(screen.getByTitle("Priority"));
    expect(screen.getByText("Priority 1")).toBeTruthy();
    fireEvent.click(screen.getByTitle("Repeat"));
    expect(screen.queryByText("Priority 1")).toBeNull();
    expect(screen.getByText("Every day")).toBeTruthy();
  });

  it("holds the page still while it's open", async () => {
    adding();
    await vi.waitFor(() => expect(mockPageScrollHeld()).toBe(true));
    cleanup();
    await vi.waitFor(() => expect(mockPageScrollHeld()).toBe(false));
  });
});

it("highlights what quick add understood and previews it", () => {
  const title = adding();
  type(title, "Call mom tomorrow p1");
  expect(marks()).toEqual(["hl hl-date:tomorrow", "hl hl-p1:p1"]);
  const chips = [...document.querySelectorAll(".quick-preview .quick-chip")].map(
    (c) => `${c.className}:${c.textContent}`,
  );
  expect(chips).toEqual(["quick-chip chip-date:Tomorrow", "quick-chip chip-priority:P1"]);
});

it("can't add a phrase on its own", () => {
  const title = adding();
  type(title, "tomorrow");
  expect((screen.getByText("Add task", { selector: "button" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.keyDown(title, { key: "Enter" });
  expect(onSubmit).not.toHaveBeenCalled();
});

it("Enter adds once, resets, and keeps the title focused", () => {
  const title = adding();
  title.focus();
  type(title, "Water plants");
  fireEvent.keyDown(title, { key: "Enter" });
  fireEvent.keyDown(title, { key: "Enter" });
  expect(onSubmit).toHaveBeenCalledTimes(1);
  expect(onSubmit.mock.calls[0]?.[0].content).toBe("Water plants");
  expect(title.value).toBe("");
  expect(document.activeElement).toBe(title);
});

it("Backspace right after a highlight turns it plain", () => {
  const title = adding();
  type(title, "Call mom tomorrow p1");
  expect(backspaceAt(title, "Call mom tomorrow".length)).toBe(false);
  expect(marks()).toEqual(["hl hl-p1:p1"]);
  expect(title.value).toBe("Call mom tomorrow p1");
  // Nothing ends there now, so the next Backspace deletes as usual.
  expect(backspaceAt(title, "Call mom tomorrow".length)).toBe(true);
});

it("un-highlights after non-ASCII text", () => {
  const title = adding();
  type(title, "Café tomorrow 🍰");
  expect(marks()).toEqual(["hl hl-date:tomorrow"]);
  expect(backspaceAt(title, "Café tomorrow".length)).toBe(false);
  expect(marks()).toEqual([]);
});

it("a picked date replaces the typed one", () => {
  const title = adding();
  type(title, "call mom tomorrow");
  fireEvent.click(screen.getByTitle("Date"));
  fireEvent.click(screen.getByText("Next week"));
  expect(title.value).toBe("call mom");
  expect(screen.getByTitle("Date").textContent).toBe("Monday");
  fireEvent.keyDown(title, { key: "Enter" });
  expect(onSubmit.mock.calls[0]?.[0]).toMatchObject({ content: "call mom", date: "2026-11-02" });
});

it("orders the pills the same on desktop, and keeps Cancel there", () => {
  adding();
  const pills = [...document.querySelectorAll<HTMLElement>(".chips .chip-btn")].map((b) => b.title);
  expect(pills).toEqual(PILLS);
  expect(screen.getByRole("button", { name: "Cancel" })).toBeTruthy();
  expect(document.querySelector("form.editor")?.className).not.toContain("on-keyboard");
});

it("a repeat chosen in the date card with no date set uses today, and returns to the card", () => {
  adding();
  fireEvent.click(screen.getByTitle("Date"));
  fireEvent.click(screen.getByText("Pick a date and time"));
  const repeatRow = () =>
    within(document.querySelector(".scheduler") as HTMLElement).getByRole("button", { name: /^Repeat/ });
  expect(repeatRow().textContent).toContain("No repeat");
  fireEvent.click(repeatRow());
  fireEvent.click(screen.getByText("Every week on Friday"));
  expect(document.querySelector(".cal-grid")).not.toBeNull();
  expect(repeatRow().textContent).toContain("Every week on Friday");
  const ringed = [...document.querySelectorAll(".cal-day.repeat-day:not(.other)")].map((d) => d.textContent);
  expect(ringed).not.toContain("29");
  // A repeat always has a date: today, by default.
  expect(document.querySelector(".cal-day.selected")?.textContent).toBe("30");
  expect(screen.getByTitle("Date").textContent).toContain("Today");
});

it("the date card's Repeat row shows a repeat typed in the title", () => {
  adding();
  const title = document.querySelector(".title-input") as HTMLTextAreaElement;
  fireEvent.change(title, { target: { value: "Water plants every week" } });
  fireEvent.click(screen.getByTitle("Date"));
  fireEvent.click(screen.getByText("Pick a date and time"));
  const row = within(document.querySelector(".scheduler") as HTMLElement).getByRole("button", {
    name: /^Repeat/,
  });
  expect(row.textContent).not.toContain("No repeat");
  expect(row.textContent).toContain("Every Fri");
});

it("the Repeat pill still works on its own", () => {
  adding();
  fireEvent.click(screen.getByTitle("Repeat"));
  fireEvent.click(screen.getByText("Every day"));
  expect(document.querySelector(".popover")).toBeNull();
  expect(screen.getByTitle("Repeat").textContent).toContain("Every day");
});

it("Escape closes the popover first, then cancels", () => {
  const title = adding();
  fireEvent.click(screen.getByTitle("Priority"));
  expect(document.querySelector(".pop-backdrop")).not.toBeNull();
  fireEvent.keyDown(title, { key: "Escape" });
  expect(document.querySelector(".popover")).toBeNull();
  expect(onCancel).not.toHaveBeenCalled();
  fireEvent.keyDown(title, { key: "Escape" });
  expect(onCancel).toHaveBeenCalledTimes(1);
});

it("picks a priority and a project from the menus", () => {
  const title = adding();
  type(title, "x");
  fireEvent.click(screen.getByTitle("Priority"));
  fireEvent.click(screen.getByText("Priority 1"));
  expect(screen.getByTitle("Priority").textContent).toBe("P1");
  fireEvent.click(screen.getByTitle("Project / section"));
  fireEvent.click(screen.getByText("Chores"));
  expect(screen.getByTitle("Project / section").textContent).toBe("Home / Chores");
  fireEvent.keyDown(title, { key: "Enter" });
  expect(onSubmit.mock.calls[0]?.[0]).toMatchObject({
    priority: "1",
    project_id: "p-home",
    section_id: "s-chores",
  });
});

it("edit mode never highlights and saves once", async () => {
  const task = fx.tasks.find((t) => t.id === "t-inbox");
  if (task === undefined) throw new Error("fixture");
  render(
    <TaskEditor
      initial={valuesFromTask({ ...task, content: "Call tomorrow p1" })}
      projects={fx.projects}
      sections={fx.sections}
      labels={fx.labels}
      keepLabel={null}
      existing={null}
      submitText="Save"
      quickAdd={false}
      keepOpen={false}
      onSubmit={onSubmit}
      onCancel={onCancel}
      onChanged={() => {}}
    />,
  );
  const title = screen.getByPlaceholderText("Task name") as HTMLTextAreaElement;
  expect(marks()).toEqual([]);
  expect(document.querySelector(".quick-preview")).toBeNull();
  await act(async () => {
    fireEvent.keyDown(title, { key: "Enter" });
    fireEvent.keyDown(title, { key: "Enter" });
  });
  expect(onSubmit).toHaveBeenCalledTimes(1);
});
