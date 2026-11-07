// The Repeat menu's typed field: it reads the text live and sets it on Enter
// or a tap on the reading. 2026-10-30, a Friday, is today.
import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { readRepeat } from "../core";
import { loadCoreForTests } from "../test/wasm";
import { DatePicker } from "./DatePicker";
import { Menu } from "./Menu";
import type { Pop } from "./pop";
import { RepeatField, type TypedRepeat } from "./RepeatField";

const TODAY = "2026-10-30";

beforeAll(() => {
  loadCoreForTests();
});

const typedRepeat = (onSubmit: (text: string) => void): TypedRepeat => ({
  read: (text) => readRepeat(text, "", TODAY)?.text ?? null,
  onSubmit,
});

const field = () => screen.getByRole("textbox", { name: "Type a repeat" }) as HTMLInputElement;
const type = (text: string) => fireEvent.change(field(), { target: { value: text } });
const reading = () => document.querySelector(".menu-reading") as HTMLButtonElement | null;

export function RepeatMenuHost({ onSubmit }: { onSubmit: (text: string) => void }) {
  const [open, setOpen] = useState<Pop>(null);
  return (
    <Menu
      id="repeat"
      title="Repeat"
      icon={null}
      text="Repeat"
      items={[
        { value: "none", label: "No repeat" },
        { value: "daily", label: "Every day" },
      ]}
      selected="none"
      open={open}
      setOpen={setOpen}
      onSelect={() => {}}
      typed={typedRepeat(onSubmit)}
    />
  );
}

export function PlainMenuHost() {
  const [open, setOpen] = useState<Pop>(null);
  return (
    <Menu
      id="priority"
      title="Priority"
      icon={null}
      text="P"
      items={[]}
      selected=""
      open={open}
      setOpen={setOpen}
      onSelect={() => {}}
    />
  );
}

export function TypedDateHost({ onSubmit }: { onSubmit: (text: string) => void }) {
  const [open, setOpen] = useState<Pop>(null);
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
        items: [{ value: "none", label: "Does not repeat" }],
        selected: "none",
        text: "Does not repeat",
        onSelect: () => {},
        typed: typedRepeat(onSubmit),
      }}
    />
  );
}

describe("RepeatField", () => {
  it("shows nothing under an empty field", () => {
    render(<RepeatField typed={typedRepeat(vi.fn())} onDone={vi.fn()} />);
    expect(field().value).toBe("");
    expect(reading()).toBeNull();
  });

  it("reads the text as it's typed", () => {
    render(<RepeatField typed={typedRepeat(vi.fn())} onDone={vi.fn()} />);
    type("every 3 tuesdays");
    expect(reading()?.textContent).toContain("Every 3 weeks on Tue");
    expect(reading()?.disabled).toBe(false);
  });

  it("says when the text isn't a repeat, and can't be set", () => {
    const onSubmit = vi.fn();
    render(<RepeatField typed={typedRepeat(onSubmit)} onDone={vi.fn()} />);
    type("whenever");
    expect(reading()?.textContent).toContain("Not a repeat");
    expect(reading()?.disabled).toBe(true);
    fireEvent.keyDown(field(), { key: "Enter" });
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("sets the text on Enter or a tap on the reading", () => {
    const onSubmit = vi.fn();
    const onDone = vi.fn();
    render(<RepeatField typed={typedRepeat(onSubmit)} onDone={onDone} />);
    type("every other friday");
    fireEvent.keyDown(field(), { key: "Enter" });
    expect(onSubmit).toHaveBeenCalledWith("every other friday");
    expect(onDone).toHaveBeenCalledTimes(1);
    fireEvent.click(reading() as HTMLButtonElement);
    expect(onSubmit).toHaveBeenCalledTimes(2);
  });

  it("isn't focused when it appears, so the keyboard waits for a tap", () => {
    render(<RepeatField typed={typedRepeat(vi.fn())} onDone={vi.fn()} />);
    expect(document.activeElement).not.toBe(field());
  });
});

describe("a Menu with a typed field", () => {
  it("puts the field above the presets", () => {
    render(<RepeatMenuHost onSubmit={vi.fn()} />);
    fireEvent.click(screen.getByTitle("Repeat"));
    const menu = document.querySelector(".popover.menu") as HTMLElement;
    expect(menu.firstElementChild?.classList.contains("menu-typed")).toBe(true);
    expect(screen.getByText("Every day")).not.toBeNull();
  });

  it("closes once a typed repeat is set", () => {
    const onSubmit = vi.fn();
    render(<RepeatMenuHost onSubmit={onSubmit} />);
    fireEvent.click(screen.getByTitle("Repeat"));
    type("every 2 weeks");
    fireEvent.keyDown(field(), { key: "Enter" });
    expect(onSubmit).toHaveBeenCalledWith("every 2 weeks");
    expect(document.querySelector(".popover.menu")).toBeNull();
  });

  it("a menu without one has no field", () => {
    render(<PlainMenuHost />);
    fireEvent.click(screen.getByTitle("Priority"));
    expect(document.querySelector(".menu-typed")).toBeNull();
  });
});

describe("the date card's Repeat step", () => {
  it("has the field too, and goes back to the card once it's set", () => {
    const onSubmit = vi.fn();
    render(<TypedDateHost onSubmit={onSubmit} />);
    fireEvent.click(screen.getByTitle("Date"));
    fireEvent.click(screen.getByText("Pick a date and time"));
    fireEvent.click(screen.getByRole("button", { name: /^Repeat/ }));
    type("every 3 tuesdays");
    fireEvent.keyDown(field(), { key: "Enter" });
    expect(onSubmit).toHaveBeenCalledWith("every 3 tuesdays");
    expect(document.querySelector(".cal-grid")).not.toBeNull();
  });
});
