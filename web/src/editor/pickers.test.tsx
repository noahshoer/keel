// The pickers on their own, each in a small host that owns the open popover
// (as the editor does). 2026-10-30, a Friday, is today.
import { act, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { Label, ReminderDraft } from "../bindings";
import { invoke } from "../ipc/invoke";
import { loadCoreForTests } from "../test/wasm";
import { DatePicker } from "./DatePicker";
import { DeadlinePicker } from "./DeadlinePicker";
import { LabelPicker } from "./LabelPicker";
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

export function LabelHost({ onToggle, onChanged }: { onToggle: (n: string) => void; onChanged: () => void }) {
  const [open, setOpen] = usePop();
  return (
    <LabelPicker
      labels={labels}
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
}: {
  onChange: (d: string, t: string) => void;
  date?: string;
  time?: string;
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
        bucket={null}
        rule={null}
        today={TODAY}
        open={open}
        setOpen={setOpen}
        onChange={onChange}
      />
    </form>
  );
}
export function DeadlineHost() {
  const [open, setOpen] = usePop();
  return (
    <DeadlinePicker
      deadline=""
      text="Deadline"
      urgent={null}
      today={TODAY}
      open={open}
      setOpen={setOpen}
      onChange={vi.fn()}
    />
  );
}
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
  it("ticks case-insensitively, toggles, and adds a new label", async () => {
    const onToggle = vi.fn();
    const onChanged = vi.fn();
    render(<LabelHost onToggle={onToggle} onChanged={onChanged} />);
    fireEvent.click(screen.getByTitle("Labels"));
    const box = screen.getByRole("checkbox") as HTMLInputElement;
    expect(box.checked).toBe(true);
    fireEvent.click(box);
    expect(onToggle).toHaveBeenCalledWith("Home");
    const field = screen.getByPlaceholderText("New label, then Enter");
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

  it("adds a preset once, removes it, and parses a custom time", () => {
    render(<ReminderHost hasDue />);
    fireEvent.click(screen.getByTitle("Reminders"));
    fireEvent.click(screen.getByText("30 min before"));
    fireEvent.click(screen.getByText("30 min before"));
    expect(out()).toEqual([{ id: null, minuteOffset: 30, at: null }]);
    fireEvent.click(screen.getByTitle("Remove reminder"));
    expect(out()).toEqual([]);
    const field = screen.getByPlaceholderText("Custom, e.g. tomorrow 9am, then Enter");
    fireEvent.change(field, { target: { value: "tomorrow" } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(screen.getByText("Needs a time, like “friday 3pm” or “in 2 days at 9am”.")).toBeTruthy();
    fireEvent.change(field, { target: { value: "tomorrow 9am" } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(out()).toEqual([{ id: null, minuteOffset: null, at: "2026-10-31T09:00" }]);
  });
});

describe("DatePicker", () => {
  const openIt = () => fireEvent.click(screen.getByTitle("Date"));

  it("quick dates and day cells pick and close", () => {
    const onChange = vi.fn();
    render(<DateHost onChange={onChange} />);
    openIt();
    expect(document.querySelector(".sched-head")?.textContent).toBe("No date");
    fireEvent.click(screen.getByText("Tomorrow"));
    expect(onChange).toHaveBeenLastCalledWith("2026-10-31", "");
    expect(document.querySelector(".scheduler")).toBeNull();
    openIt();
    fireEvent.click(screen.getByText("12", { selector: ".cal-day:not(.other)" }));
    expect(onChange).toHaveBeenLastCalledWith("2026-10-12", "");
  });

  it("month arrows move the calendar", () => {
    render(<DateHost onChange={vi.fn()} />);
    openIt();
    expect(document.querySelector(".cal-title")?.textContent).toBe("October 2026");
    fireEvent.click(screen.getByTitle("Next month"));
    expect(document.querySelector(".cal-title")?.textContent).toBe("November 2026");
    fireEvent.click(screen.getByTitle("Previous month"));
    fireEvent.click(screen.getByTitle("Previous month"));
    expect(document.querySelector(".cal-title")?.textContent).toBe("September 2026");
    fireEvent.click(screen.getByTitle("Today"));
    expect(document.querySelector(".cal-title")?.textContent).toBe("October 2026");
  });

  it("a typed time picks today with it; a bad one is marked", () => {
    const onChange = vi.fn();
    render(<DateHost onChange={onChange} />);
    openIt();
    const field = screen.getByPlaceholderText("Add time, e.g. 5pm or 17:30");
    fireEvent.change(field, { target: { value: "bad" } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(field.className).toContain("bad");
    fireEvent.change(field, { target: { value: "5pm" } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(onChange).toHaveBeenLastCalledWith("2026-10-30", "17:00");
    expect(document.querySelector(".scheduler")).toBeNull();
  });

  it("uses the native time field on touch screens", () => {
    vi.stubGlobal("matchMedia", (q: string) => ({ matches: q === "(pointer: coarse)" }));
    const onChange = vi.fn();
    render(<DateHost onChange={onChange} date="2026-11-02" />);
    openIt();
    const field = document.querySelector("input.time-input.native") as HTMLInputElement;
    expect(field.type).toBe("time");
    expect(screen.getByText("Add time")).toBeTruthy();
    fireEvent.change(field, { target: { value: "07:05" } });
    expect(onChange).toHaveBeenLastCalledWith("2026-11-02", "07:05");
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
    fireEvent.click(screen.getByTitle("Choose month and year"));
    expect(document.querySelector(".cal-grid")).toBeNull();
    expect(document.querySelector(".wheel-col.loop .wheel-item.on")?.textContent).toBe("October");
    expect(document.querySelector('.wheel-col[data-col="1"] .wheel-item.on')?.textContent).toBe("2026");
  });
});

describe("DeadlinePicker", () => {
  it("has no time row and its own empty header", () => {
    render(<DeadlineHost />);
    fireEvent.click(screen.getByTitle("Deadline: the date it must be done by"));
    expect(document.querySelector(".sched-head")?.textContent).toBe("No deadline");
    expect(document.querySelector(".sched-time")).toBeNull();
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
