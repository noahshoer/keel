// The pickers on their own, each in a small host that owns the open popover
// (as the editor does). 2026-10-30, a Friday, is today.
import { act, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { DueBucket, Label, ReminderDraft } from "../bindings";
import { invoke } from "../ipc/invoke";
import { CheckCircleIcon } from "../shell/icons";
import { stubMedia } from "../test/media";
import { loadCoreForTests } from "../test/wasm";
import { DatePicker } from "./DatePicker";
import { DeadlinePicker } from "./DeadlinePicker";
import { PRIORITY_ITEMS } from "./items";
import { LabelPicker } from "./LabelPicker";
import { LABEL_COLORS } from "./labelColors";
import { Menu } from "./Menu";
import type { Pop } from "./pop";
import { ReminderPicker } from "./ReminderPicker";

const TODAY = "2026-10-30";

beforeAll(() => {
  loadCoreForTests();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

/** The drawing of the check-circle, to compare a picker's mark with. */
function checkCircle(): string {
  const { container, unmount } = render(<CheckCircleIcon />);
  const drawing = container.querySelector("svg")?.innerHTML ?? "";
  unmount();
  return drawing;
}

function usePop(): [Pop, (p: Pop) => void] {
  return useState<Pop>(null);
}

export function MenuHost({ onSelect }: { onSelect: (v: string) => void }) {
  const [open, setOpen] = usePop();
  return (
    <Menu
      id="priority"
      title="Priority"
      icon={null}
      text="Priority"
      items={[
        { value: "1", label: "Priority 1" },
        { value: "4", label: "Priority 4" },
      ]}
      selected="4"
      open={open}
      setOpen={setOpen}
      onSelect={onSelect}
    />
  );
}
const labels: Label[] = [
  { id: "l1", name: "Home", color: "grey", item_order: 0, is_favorite: false, is_deleted: false },
];

export function LabelHost({
  onToggle = () => {},
  onChanged = () => {},
  initial = labels,
}: {
  onToggle?: (n: string) => void;
  onChanged?: () => void;
  initial?: Label[];
}) {
  const [open, setOpen] = usePop();
  return (
    <LabelPicker
      labels={initial}
      selected={["home"]}
      text="home"
      open={open}
      setOpen={setOpen}
      onToggle={onToggle}
      onChanged={onChanged}
    />
  );
}
export function ReminderHost({ hasDue, start = [] }: { hasDue: boolean; start?: ReminderDraft[] }) {
  const [open, setOpen] = usePop();
  const [drafts, setDrafts] = useState<ReminderDraft[]>(start);
  return (
    <>
      <ReminderPicker
        reminders={drafts}
        labels={drafts.map((d) => (d.minuteOffset !== null ? `${d.minuteOffset}m` : (d.at ?? "")))}
        text="Reminders"
        hasDue={hasDue}
        today={TODAY}
        open={open}
        setOpen={setOpen}
        onChange={setDrafts}
      />
      <output>{JSON.stringify(drafts)}</output>
    </>
  );
}
export function DateHost({
  onChange,
  date = "",
  time = "",
  bucket = null,
}: {
  onChange: (d: string, t: string) => void;
  date?: string;
  time?: string;
  bucket?: DueBucket | null;
}) {
  const [open, setOpen] = usePop();
  return (
    <form
      onKeyDown={(e) => {
        if (e.key === "Escape") onChange("escaped", "");
      }}
    >
      <DatePicker
        date={date}
        time={time}
        text="Date"
        bucket={bucket}
        rule={null}
        today={TODAY}
        open={open}
        setOpen={setOpen}
        onChange={onChange}
      />
    </form>
  );
}
export function RepeatDateHost() {
  const [open, setOpen] = usePop();
  const [value, setValue] = useState("none");
  const items = [
    { value: "none", label: "Does not repeat" },
    { value: "daily", label: "Every day" },
  ];
  return (
    <DatePicker
      date=""
      time=""
      text="Date"
      bucket={null}
      rule={null}
      today={TODAY}
      open={open}
      setOpen={setOpen}
      onChange={() => {}}
      repeat={{
        items,
        selected: value,
        text: items.find((i) => i.value === value)?.label ?? "",
        onSelect: setValue,
      }}
    />
  );
}
export function DeadlineHost({ deadline = "" }: { deadline?: string }) {
  const [open, setOpen] = usePop();
  return (
    <DeadlinePicker
      deadline={deadline}
      text="Deadline"
      urgent={null}
      today={TODAY}
      open={open}
      setOpen={setOpen}
      onChange={vi.fn()}
    />
  );
}
describe("Priority's menu", () => {
  it("colors the chosen row, text and icon, by its priority", () => {
    render(
      <Menu
        id="priority"
        title="Priority"
        icon={null}
        text="P1"
        items={PRIORITY_ITEMS}
        selected="1"
        open="priority"
        setOpen={() => {}}
        onSelect={() => {}}
      />,
    );
    const row = (name: string) => screen.getByText(name).closest("button")?.className;
    expect(row("Priority 1")).toContain("row-p1");
    expect(row("Priority 2")).not.toContain("row-p2");
  });
});

describe("Menu", () => {
  it("opens, ticks the selected item, and closes on a pick", () => {
    const onSelect = vi.fn();
    render(<MenuHost onSelect={onSelect} />);
    fireEvent.click(screen.getByTitle("Priority"));
    expect(screen.getByTitle("Priority").className).toContain("chip-open");
    expect(screen.getByText("Priority 4").closest(".menu-item")?.className).toContain("checked");
    fireEvent.click(screen.getByText("Priority 1"));
    expect(onSelect).toHaveBeenCalledWith("1");
    expect(document.querySelector(".popover")).toBeNull();
  });
});

describe("LabelPicker", () => {
  it("colors its pill by the last chosen label, open or closed", () => {
    render(
      <LabelPicker
        labels={[
          { ...labels[0], id: "a", name: "first", color: "red" } as Label,
          { ...labels[0], id: "b", name: "last", color: "blue" } as Label,
        ]}
        selected={["first", "last"]}
        text="2 labels"
        open="labels"
        setOpen={() => {}}
        onToggle={() => {}}
        onChanged={() => {}}
      />,
    );
    const pill = screen.getByTitle("Labels");
    expect(pill.style.color).toBe("var(--label-blue)");
    expect(pill.className).toContain("pill-tinted");
  });

  it("a chosen label's row reads in its color; a label with none stays plain", () => {
    render(
      <LabelPicker
        labels={[
          { ...labels[0], id: "a", name: "red one", color: "red" } as Label,
          { ...labels[0], id: "b", name: "plain one", color: "charcoal" } as Label,
          { ...labels[0], id: "c", name: "unchosen", color: "blue" } as Label,
        ]}
        selected={["red one", "plain one"]}
        text="2 labels"
        open="labels"
        setOpen={() => {}}
        onToggle={() => {}}
        onChanged={() => {}}
      />,
    );
    const row = (name: string) => screen.getByText(name).closest(".label-row") as HTMLElement;
    expect(row("red one").style.color).toBe("var(--label-red)");
    expect(row("plain one").style.color).toBe("");
    expect(row("unchosen").style.color).toBe("");
  });

  it("leaves its pill plain with nothing chosen", () => {
    render(
      <LabelPicker
        labels={[{ ...labels[0], color: "red" } as Label]}
        selected={[]}
        text="Labels"
        open={null}
        setOpen={() => {}}
        onToggle={() => {}}
        onChanged={() => {}}
      />,
    );
    expect(screen.getByTitle("Labels").style.color).toBe("");
  });

  it("has the find field first, and marks chosen labels", () => {
    render(<LabelHost />);
    fireEvent.click(screen.getByTitle("Labels"));
    const pop = document.querySelector(".popover") as HTMLElement;
    expect(pop.firstElementChild?.getAttribute("placeholder")).toBe("Find or add a label");
    const row = document.querySelector(".label-row") as HTMLElement;
    expect(row.className).toContain("on");
    expect(row.querySelector(".menu-check svg")?.innerHTML).toBe(checkCircle());
  });

  it("shows each label's tag in its color, unknown colors as the default", async () => {
    const { container } = render(
      <LabelHost
        initial={[
          { ...labels[0], id: "a", name: "red one", color: "red" } as Label,
          { ...labels[0], id: "b", name: "odd one", color: "mystery" } as Label,
        ]}
      />,
    );
    fireEvent.click(screen.getByTitle("Labels"));
    const tags = [...container.querySelectorAll<HTMLElement>(".label-tag-icon")];
    expect(tags.map((t) => t.style.color)).toEqual(["var(--label-red)", "var(--label-charcoal)"]);
    expect(tags.every((t) => t.querySelector("svg") !== null)).toBe(true);
  });

  it("finds labels as you type", () => {
    const errand = { ...labels[0], id: "e", name: "errand" } as Label;
    render(<LabelHost initial={[...labels, errand]} />);
    fireEvent.click(screen.getByTitle("Labels"));
    fireEvent.change(screen.getByPlaceholderText("Find or add a label"), { target: { value: "ERR" } });
    expect(screen.queryByText("Home")).toBeNull();
    expect(screen.getByText("errand")).toBeTruthy();
  });

  it("chooses a label's color from the twelve, with a way back", async () => {
    const saved = (await invoke("list_labels", {}))[0];
    if (!saved) throw new Error("no label");
    const { container } = render(<LabelHost initial={[{ ...saved, color: "orange" }]} />);
    fireEvent.click(screen.getByTitle("Labels"));
    fireEvent.click(screen.getByRole("button", { name: `Color for ${saved.name}` }));
    const swatches = [...container.querySelectorAll<HTMLButtonElement>(".label-swatch")];
    expect(swatches).toHaveLength(12);
    expect(swatches.map((b) => b.getAttribute("aria-label"))).toEqual(LABEL_COLORS.map((c) => c.name));
    expect(container.querySelector(".label-swatch.selected")?.getAttribute("aria-label")).toBe("Orange");

    fireEvent.click(screen.getByRole("button", { name: "Back to labels" }));
    expect(container.querySelector(".label-swatch")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: `Color for ${saved.name}` }));
    fireEvent.click(screen.getByRole("button", { name: "Teal" }));
    expect(container.querySelector(".label-swatch")).toBeNull();
    await vi.waitFor(async () => {
      const after = (await invoke("list_labels", {})).find((l) => l.id === saved.id);
      expect(after?.color).toBe("teal");
    });
  });

  it("ticks case-insensitively, toggles, and adds a new label", async () => {
    const onToggle = vi.fn();
    const onChanged = vi.fn();
    render(<LabelHost onToggle={onToggle} onChanged={onChanged} />);
    fireEvent.click(screen.getByTitle("Labels"));
    const box = screen.getByRole("checkbox") as HTMLInputElement;
    expect(box.checked).toBe(true);
    fireEvent.click(box);
    expect(onToggle).toHaveBeenCalledWith("Home");
    const field = screen.getByPlaceholderText("Find or add a label");
    fireEvent.change(field, { target: { value: "Garden" } });
    await act(async () => {
      fireEvent.keyDown(field, { key: "Enter" });
    });
    expect(onToggle).toHaveBeenCalledWith("Garden");
    expect(onChanged).toHaveBeenCalled();
    expect((await invoke("list_labels", {})).map((l) => l.name)).toContain("Garden");
  });
});

describe("ReminderPicker", () => {
  const out = () => JSON.parse(document.querySelector("output")?.textContent ?? "[]") as ReminderDraft[];

  it("needs a due for the presets", () => {
    render(<ReminderHost hasDue={false} />);
    fireEvent.click(screen.getByTitle("Reminders"));
    expect(screen.getByText("Give the task a date to use these:")).toBeTruthy();
    expect((screen.getByText("At time of task").closest("button") as HTMLButtonElement).disabled).toBe(true);
  });

  it("has the custom field first, then the presets", () => {
    render(<ReminderHost hasDue />);
    fireEvent.click(screen.getByTitle("Reminders"));
    const pop = document.querySelector(".popover") as HTMLElement;
    expect(pop.firstElementChild?.getAttribute("placeholder")).toBe("Custom, e.g. tomorrow 9am");
  });

  it("toggles presets and parses a custom time, which can be removed", () => {
    render(<ReminderHost hasDue />);
    fireEvent.click(screen.getByTitle("Reminders"));
    fireEvent.click(screen.getByText("30 min before"));
    expect(out()).toEqual([{ id: null, minuteOffset: 30, at: null }]);
    expect(screen.getByText("30 min before").closest("button")?.className).toContain("checked");
    fireEvent.click(screen.getByText("At time of task"));
    expect(out()).toHaveLength(2);
    fireEvent.click(screen.getByText("At time of task"));
    fireEvent.click(screen.getByText("30 min before"));
    expect(out()).toEqual([]);
    const field = screen.getByPlaceholderText("Custom, e.g. tomorrow 9am");
    fireEvent.change(field, { target: { value: "tomorrow" } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(screen.getByText("Needs a time, like “friday 3pm” or “in 2 days at 9am”.")).toBeTruthy();
    fireEvent.change(field, { target: { value: "tomorrow 9am" } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(out()).toEqual([{ id: null, minuteOffset: null, at: "2026-10-31T09:00" }]);
    fireEvent.click(screen.getByTitle("Remove reminder"));
    expect(out()).toEqual([]);
  });
});

describe("DatePicker", () => {
  const openIt = () => fireEvent.click(screen.getByTitle("Date"));

  const calendar = () => fireEvent.click(screen.getByText("Pick a date and time"));

  it("opens on the quick choices; each picks and closes", () => {
    const onChange = vi.fn();
    render(<DateHost onChange={onChange} />);
    openIt();
    expect([...document.querySelectorAll(".scheduler .menu-label")].map((e) => e.textContent)).toEqual([
      "Today",
      "Tomorrow",
      "This weekend",
      "Next week",
      "Pick a date and time",
      "No date",
    ]);
    expect(document.querySelector(".cal-grid")).toBeNull();
    fireEvent.click(screen.getByText("Tomorrow"));
    expect(onChange).toHaveBeenLastCalledWith("2026-10-31", "");
    expect(document.querySelector(".scheduler")).toBeNull();
    openIt();
    fireEvent.click(screen.getByText("No date"));
    expect(onChange).toHaveBeenLastCalledWith("", "");
    expect(document.querySelector(".scheduler")).toBeNull();
  });

  it("colors each quick choice by when it falls", () => {
    render(<DateHost onChange={vi.fn()} />);
    openIt();
    const row = (name: string) => screen.getByText(name).closest("button")?.className;
    // Friday 2026-10-30: today, tomorrow, and a Saturday and Monday this coming week.
    expect(row("Today")).toContain("quick-today");
    expect(row("Tomorrow")).toContain("quick-tomorrow");
    expect(row("Next week")).toContain("quick-week");
    expect(row("No date")).not.toContain("quick-");
    expect(row("Pick a date and time")).not.toContain("quick-");
  });

  it("the open pill takes its date's color class", () => {
    render(<DateHost onChange={vi.fn()} date="2026-10-31" bucket="Tomorrow" />);
    openIt();
    const pill = screen.getByTitle("Date").className;
    expect(pill).toContain("chip-open");
    expect(pill).toContain("tomorrow");
  });

  it("offers no No date while it repeats, since a repeat always has a date", () => {
    render(
      <DatePicker
        date="2026-10-30"
        time=""
        text="Today"
        bucket="Today"
        rule="Daily"
        today={TODAY}
        open="date"
        setOpen={() => {}}
        onChange={() => {}}
      />,
    );
    expect(screen.queryByText("No date")).toBeNull();
    expect(screen.getByText("Pick a date and time")).toBeTruthy();
  });

  it("marks the quick choice that's set", () => {
    render(<DateHost onChange={vi.fn()} date="2026-10-31" />);
    openIt();
    expect(document.querySelector(".menu-item.checked .menu-label")?.textContent).toBe("Tomorrow");
  });

  it("Pick a date and time shows the calendar, a Time row and the Repeat row", () => {
    render(<RepeatDateHost />);
    openIt();
    calendar();
    expect(document.querySelector(".cal-grid")).not.toBeNull();
    expect(document.querySelector(".sched-time")?.textContent).toContain("Time");
    expect(screen.getByRole("button", { name: /^Repeat/ }).textContent).toContain("Does not repeat");
    expect(document.querySelector(".sched-quick")).toBeNull();
  });

  const otherDays = (n: string) =>
    [...document.querySelectorAll(".cal-day.other")].filter((d) => d.textContent === n);

  it("days outside the shown month are grayed, and only they are", () => {
    render(<DateHost onChange={vi.fn()} />);
    openIt();
    calendar();
    expect(document.querySelectorAll(".cal-day:not(.other)")).toHaveLength(31);
    expect(otherDays("1").length).toBeGreaterThan(0);
  });

  it("a week shared by two months is drawn once, in one run of weeks", () => {
    render(<DateHost onChange={vi.fn()} />);
    openIt();
    calendar();
    // October 2026 starts on a Thursday, so its first week opens with late September.
    const week = document.querySelector('[data-month="2026-10"]') as HTMLElement;
    const days = [...week.querySelectorAll(".cal-day")];
    expect(days).toHaveLength(7);
    expect(days[4]?.textContent).toBe("1");
    expect(days[4]?.className).not.toContain("other");
    expect(days[3]?.className).toContain("other");
    expect(document.querySelector(".cal-month")).toBeNull();
    const next = week.nextElementSibling as HTMLElement;
    const rowHeight = Number.parseFloat(next.style.top) - Number.parseFloat(week.style.top);
    expect(next.querySelector(".cal-day")?.textContent).toBe("4");
    expect(rowHeight).toBeGreaterThan(0);
  });

  it("a grayed day can be picked", () => {
    const onChange = vi.fn();
    render(<DateHost onChange={onChange} />);
    openIt();
    calendar();
    const week = document.querySelector('[data-month="2026-10"]') as HTMLElement;
    fireEvent.click(week.querySelectorAll(".cal-day")[3] as HTMLElement);
    expect(onChange).toHaveBeenLastCalledWith("2026-09-30", "");
  });

  it("a day picks and stays open, so a time or repeat can follow", () => {
    const onChange = vi.fn();
    render(<DateHost onChange={onChange} />);
    openIt();
    calendar();
    fireEvent.click(screen.getByText("12", { selector: ".cal-day:not(.other)" }));
    expect(onChange).toHaveBeenLastCalledWith("2026-10-12", "");
    expect(document.querySelector(".scheduler")).not.toBeNull();
  });

  it("Repeat opens its options with a way back", () => {
    render(<RepeatDateHost />);
    openIt();
    calendar();
    fireEvent.click(screen.getByRole("button", { name: /^Repeat/ }));
    expect(document.querySelector(".cal-grid")).toBeNull();
    expect(screen.getByText("Every day")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Back to date" }));
    expect(document.querySelector(".cal-grid")).not.toBeNull();
  });

  it("choosing a repeat returns to the card with the new text, still open", () => {
    render(<RepeatDateHost />);
    openIt();
    calendar();
    fireEvent.click(screen.getByRole("button", { name: /^Repeat/ }));
    fireEvent.click(screen.getByText("Every day"));
    expect(document.querySelector(".scheduler")).not.toBeNull();
    expect(document.querySelector(".cal-grid")).not.toBeNull();
    expect(screen.getByRole("button", { name: /^Repeat/ }).textContent).toContain("Every day");
  });

  it("has no Repeat row when the caller gives no repeat", () => {
    render(<DateHost onChange={vi.fn()} />);
    openIt();
    calendar();
    expect(screen.queryByRole("button", { name: /^Repeat/ })).toBeNull();
  });

  const scroller = () => document.querySelector(".cal-scroll") as HTMLElement;
  const topOf = (month: string) =>
    Number.parseFloat((document.querySelector(`[data-month="${month}"]`) as HTMLElement).style.top);
  const scrollTo = (month: string) => {
    scroller().scrollTop = topOf(month);
    fireEvent.scroll(scroller());
  };
  const title = () => document.querySelector(".cal-title")?.textContent;

  it("a month lights up once it fills the lower half of the view, before reaching the top", () => {
    render(<DateHost onChange={vi.fn()} />);
    openIt();
    calendar();
    const november = document.querySelector('[data-month="2026-11"]') as HTMLElement;
    const row =
      Number.parseFloat((november.nextElementSibling as HTMLElement).style.top) -
      Number.parseFloat(november.style.top);
    scroller().scrollTop = topOf("2026-11") - row * 3;
    fireEvent.scroll(scroller());
    expect(title()).toBe("November 2026");
    expect(document.querySelectorAll(".cal-day:not(.other)")).toHaveLength(30);
  });

  it("scrolling the calendar moves the title, with no month arrows", () => {
    render(<DateHost onChange={vi.fn()} />);
    openIt();
    calendar();
    expect(title()).toBe("October 2026");
    expect(screen.queryByTitle("Next month")).toBeNull();
    expect(screen.queryByTitle("Previous month")).toBeNull();
    scrollTo("2026-11");
    expect(title()).toBe("November 2026");
    scrollTo("2026-12");
    expect(title()).toBe("December 2026");
  });

  it("the gray follows the month that's scrolled to", () => {
    render(<DateHost onChange={vi.fn()} />);
    openIt();
    calendar();
    scrollTo("2026-11");
    expect(document.querySelectorAll(".cal-day:not(.other)")).toHaveLength(30);
    const week = document.querySelector('[data-month="2026-11"]') as HTMLElement;
    expect(week.querySelector(".cal-day")?.className).not.toContain("other");
  });

  it("opens scrolled to the selected month", () => {
    render(<DeadlineHost deadline="2026-12-25" />);
    fireEvent.click(screen.getByTitle("Deadline: the date it must be done by"));
    expect(title()).toBe("December 2026");
    expect(scroller().scrollTop).toBe(topOf("2026-12"));
    expect(scroller().scrollTop).toBeGreaterThan(0);
  });

  it("closing the month / year chooser returns to the shown month", () => {
    render(<DateHost onChange={vi.fn()} />);
    openIt();
    calendar();
    scrollTo("2026-11");
    fireEvent.click(screen.getByTitle("Choose month and year"));
    fireEvent.click(screen.getByTitle("Choose month and year"));
    expect(title()).toBe("November 2026");
    expect(scroller().scrollTop).toBe(topOf("2026-11"));
  });

  it("marks a chosen quick date with the check-circle", () => {
    render(<DateHost onChange={vi.fn()} date="2026-10-31" />);
    openIt();
    expect(document.querySelector(".menu-check svg")?.innerHTML).toBe(checkCircle());
  });

  it("a typed time picks today with it; a bad one is marked", () => {
    const onChange = vi.fn();
    render(<DateHost onChange={onChange} />);
    openIt();
    calendar();
    const field = screen.getByPlaceholderText("Add time, e.g. 5pm or 17:30");
    fireEvent.change(field, { target: { value: "bad" } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(field.className).toContain("bad");
    fireEvent.change(field, { target: { value: "5pm" } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(onChange).toHaveBeenLastCalledWith("2026-10-30", "17:00");
    expect(document.querySelector(".scheduler")).toBeNull();
  });

  describe("on touch screens", () => {
    const wheelOpen = (date: string, onChange: (d: string, t: string) => void, time = "") => {
      stubMedia({ coarse: true });
      render(<DateHost onChange={onChange} date={date} time={time} />);
      openIt();
      calendar();
    };
    const timeRow = () => screen.getByRole("button", { name: /^Time/ });

    it("has no text field for the time, which would take focus from the title", () => {
      wheelOpen("2026-11-02", vi.fn());
      expect(document.querySelector("input.time-input")).toBeNull();
      expect(timeRow().textContent).toContain("Add time");
      expect(fireEvent.mouseDown(timeRow())).toBeTruthy();
    });

    it("opening the wheel saves nothing; it starts at now, and Set time takes it", () => {
      vi.useFakeTimers({ toFake: ["Date"] });
      vi.setSystemTime(new Date(2026, 10, 2, 15, 34));
      const onChange = vi.fn();
      wheelOpen("2026-11-02", onChange);
      fireEvent.click(timeRow());
      vi.useRealTimers();
      expect(onChange).not.toHaveBeenCalled();
      expect(document.querySelectorAll(".time-wheel .wheel-col")).toHaveLength(3);
      const minutes = document.querySelector('.time-wheel [data-col="minute"]') as HTMLElement;
      expect(minutes.scrollTop).toBe(34 * 36);
      expect(screen.queryByRole("button", { name: "Remove time" })).toBeNull();
      fireEvent.click(screen.getByRole("button", { name: "Set time" }));
      expect(onChange).toHaveBeenLastCalledWith("2026-11-02", "15:34");
    });

    it("scrolling from an empty time sets what the wheel shows", async () => {
      vi.useFakeTimers({ toFake: ["Date"] });
      vi.setSystemTime(new Date(2026, 10, 2, 15, 34));
      const onChange = vi.fn();
      wheelOpen("2026-11-02", onChange);
      fireEvent.click(timeRow());
      vi.useRealTimers();
      const minutes = document.querySelector('.time-wheel [data-col="minute"]') as HTMLElement;
      minutes.scrollTop = 40 * 36;
      fireEvent.scroll(minutes);
      await vi.waitFor(() => expect(onChange).toHaveBeenLastCalledWith("2026-11-02", "15:40"));
    });

    it("scrolling a column changes the time", async () => {
      const onChange = vi.fn();
      wheelOpen("2026-11-02", onChange, "07:05");
      fireEvent.click(timeRow());
      const minutes = document.querySelector('.time-wheel [data-col="minute"]') as HTMLElement;
      expect(minutes.scrollTop).toBe(5 * 36);
      minutes.scrollTop = 20 * 36;
      fireEvent.scroll(minutes);
      await vi.waitFor(() => expect(onChange).toHaveBeenLastCalledWith("2026-11-02", "07:20"));
    });

    it("removes the time", () => {
      const onChange = vi.fn();
      wheelOpen("2026-11-02", onChange, "07:05");
      expect(timeRow().textContent).toContain("7:05 AM");
      fireEvent.click(timeRow());
      fireEvent.click(screen.getByRole("button", { name: "Remove time" }));
      expect(onChange).toHaveBeenLastCalledWith("2026-11-02", "");
    });
  });

  it("Escape closes the scheduler without reaching the form", () => {
    const onChange = vi.fn();
    render(<DateHost onChange={onChange} />);
    openIt();
    fireEvent.keyDown(screen.getByText("Tomorrow"), { key: "Escape" });
    expect(document.querySelector(".scheduler")).toBeNull();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("the chooser shows month and year wheels", () => {
    render(<DateHost onChange={vi.fn()} />);
    openIt();
    calendar();
    fireEvent.click(screen.getByTitle("Choose month and year"));
    expect(document.querySelector(".cal-grid")).toBeNull();
    expect(document.querySelector(".wheel-col.loop .wheel-item.on")?.textContent).toBe("October");
    expect(document.querySelector('.wheel-col[data-col="1"] .wheel-item.on')?.textContent).toBe("2026");
  });
});

describe("DeadlinePicker", () => {
  const openIt = () => fireEvent.click(screen.getByTitle("Deadline: the date it must be done by"));

  // The task view's, which has no X; the task card's is in editor.test.tsx.
  it("is a calendar and No deadline, with no Time or Repeat row", () => {
    render(<DeadlineHost />);
    openIt();
    expect(document.querySelector(".cal-grid")).not.toBeNull();
    expect(document.querySelector(".sched-time")).toBeNull();
    expect(screen.queryByText("Pick a date and time")).toBeNull();
    expect(screen.queryByRole("button", { name: /^Repeat/ })).toBeNull();
    expect(screen.getByText("No deadline")).toBeTruthy();
  });

  it("marks its day in the plain selected color, not a date's bucket color", () => {
    render(<DeadlineHost deadline="2026-12-25" />);
    openIt();
    const day = document.querySelector(".cal-day.selected") as HTMLElement;
    expect(day.textContent).toBe("25");
    expect(day.className).not.toContain("sel-");
  });

  it("a day picks and closes", () => {
    render(<DeadlineHost />);
    openIt();
    fireEvent.click(screen.getByText("12", { selector: ".cal-day:not(.other)" }));
    expect(document.querySelector(".scheduler")).toBeNull();
  });
});

// The task view's faces: a field row's and a dotted pill's.
export function FacedDateHost({ onChange }: { onChange: (d: string, t: string) => void }) {
  const [open, setOpen] = usePop();
  return (
    <DatePicker
      date="2026-10-30"
      time=""
      text="Today"
      bucket="Today"
      rule="Daily"
      today={TODAY}
      open={open}
      setOpen={setOpen}
      onChange={onChange}
      face={{ className: "tv-field-btn due-chip today", text: "Today", icon: true }}
      icon={<span className="filled" />}
    />
  );
}

export function PillHost() {
  const [open, setOpen] = usePop();
  return (
    <Menu
      id="priority"
      title="Priority"
      icon={<span className="flag" />}
      text="Priority"
      items={[{ value: "1", label: "Priority 1" }]}
      selected="4"
      open={open}
      setOpen={setOpen}
      onSelect={vi.fn()}
      face={{ className: "tv-pill", text: "+ Priority", icon: false }}
    />
  );
}

describe("Face", () => {
  it("a DatePicker with a face shows its class, icon and text, no ⟳, and still opens", () => {
    const onChange = vi.fn();
    render(<FacedDateHost onChange={onChange} />);
    const btn = screen.getByTitle("Date");
    expect(btn.className).toBe("tv-field-btn due-chip today");
    expect(btn.textContent).toBe("Today");
    expect(btn.querySelector(".filled")).not.toBeNull();
    // One icon (the face's), no repeat mark.
    expect(btn.querySelectorAll("svg")).toHaveLength(0);
    fireEvent.click(btn);
    expect(btn.className).toBe("tv-field-btn due-chip today chip-open");
    fireEvent.click(screen.getByText("Tomorrow"));
    expect(onChange).toHaveBeenLastCalledWith("2026-10-31", "");
  });

  it("a pill face has no icon and no caret", () => {
    render(<PillHost />);
    const btn = screen.getByTitle("Priority");
    expect(btn.className).toBe("tv-pill");
    expect(btn.innerHTML).toBe("+ Priority");
  });
});
