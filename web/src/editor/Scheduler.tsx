// The date popover, in steps: quick choices, then a month grid (or month / year
// wheels) with Time and Repeat rows, then the repeat options. A deadline is the
// grid alone. keel-core builds the grid and reads typed times; this draws them.
import { useEffect, useRef, useState } from "react";
import type { Recurrence } from "../bindings";
import {
  dueBucket,
  monthNames,
  monthView,
  quickDates,
  shiftMonth,
  timeLabel,
  typedTime,
  yearChoices,
} from "../core";
import { BUCKET_CLASS } from "../lists/chips";
import {
  CheckSmallIcon,
  ChevronDownSmallIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  XIcon,
  XSmallIcon,
} from "../shell/icons";
import { isTouch, weekStart } from "./locale";
import type { MenuItem } from "./Menu";
import { cx } from "./pop";
import { usePlacedPopover } from "./popover";
import { TimeWheel } from "./TimeWheel";
import { installMonthWheel, installYearWheel } from "./wheel";

/** The repeat choices the date card offers: the same ones as the Repeat pill. */
export type CalendarActions = { onDone: () => void; onCancel: () => void };

export type RepeatChoice = {
  items: MenuItem[];
  selected: string;
  /** The Repeat row's text. */
  text: string;
  onSelect: (value: string) => void;
};

/** `[year, month]` (1-based) of a `YYYY-MM-DD` date. */
function yearMonth(date: string): [number, number] {
  return [Number(date.slice(0, 4)), Number(date.slice(5, 7))];
}

export function Scheduler({
  selected,
  time,
  withTime = true,
  emptyText = "No date",
  rule,
  repeat,
  today,
  onPick,
  close,
  onCalendar,
  actions,
}: {
  /** `YYYY-MM-DD` or `null`. */
  selected: string | null;
  /** `HH:MM` or `null`. */
  time: string | null;
  /** A date with its steps and Time row (off for deadlines, which are the grid alone). */
  withTime?: boolean;
  /** The choice that clears it. */
  emptyText?: string;
  /** The repeat rule whose days are ringed. */
  rule: Recurrence | null;
  /** Adds the Repeat row and its options. */
  repeat?: RepeatChoice | undefined;
  today: string;
  /** A date (`null` for none) and a time (`HH:MM` or ""). */
  onPick: (date: string | null, time: string) => void;
  close: () => void;
  /** When the quick choices give way to the calendar. */
  onCalendar?: (() => void) | undefined;
  /** The calendar's check and X; with them a day selects and stays open. */
  actions?: CalendarActions | undefined;
}) {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  const [step, setStep] = useState<"quick" | "card" | "repeat">(withTime ? "quick" : "card");
  const [month, setMonth] = useState<[number, number]>(() => yearMonth(selected ?? today));
  const monthRef = useRef(month);
  monthRef.current = month;
  // The month / year wheels, opened from the title. The years are fixed
  // while they're open, so a wheel's rows don't shift under it.
  const [choosing, setChoosing] = useState(false);
  const [wheelYears, setWheelYears] = useState<number[]>([]);
  const wheel = useRef<HTMLDivElement>(null);
  const [timeText, setTimeText] = useState(() => (time === null ? "" : timeLabel(time)));
  const [timeBad, setTimeBad] = useState(false);
  const [touch] = useState(isTouch);
  const [start] = useState(weekStart);
  const timeStr = time ?? "";

  useEffect(() => {
    if (!choosing) return;
    const [y, mo] = monthRef.current;
    const years = yearChoices(today, y);
    setWheelYears(years);
    const col = wheel.current?.querySelectorAll<HTMLElement>(".wheel-col");
    const months = col?.[0];
    const yearsCol = col?.[1];
    const stops: Array<() => void> = [];
    if (months) stops.push(installMonthWheel(months, mo - 1, (m0) => setMonth(([yy]) => [yy, m0 + 1])));
    if (yearsCol) {
      // The year rows render with the years set above; wait for them.
      const frame = requestAnimationFrame(() => {
        stops.push(
          installYearWheel(yearsCol, Math.max(0, years.indexOf(y)), (i) => {
            const year = years[i];
            if (year !== undefined) setMonth(([, m]) => [year, m]);
          }),
        );
      });
      stops.push(() => cancelAnimationFrame(frame));
    }
    return () => {
      for (const stop of stops) stop();
    };
  }, [choosing, today]);

  const [y, mo] = month;
  const view = monthView(y, mo, start, today, selected, rule);

  const commitTime = (): boolean => {
    if (!timeText.trim()) {
      setTimeBad(false);
      onPick(selected, "");
      return true;
    }
    const t = typedTime(timeText);
    if (t === null) {
      setTimeBad(true);
      return false;
    }
    setTimeBad(false);
    setTimeText(timeLabel(t));
    // A time without a date means today.
    onPick(selected ?? today, t);
    return true;
  };

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: Escape anywhere in the popover closes it, not the editor (the Dioxus markup).
    <div
      ref={ref}
      className="popover scheduler"
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          close();
        }
      }}
    >
      {step === "quick" ? (
        <div className="sched-quick">
          {quickDates(today).map(([label, hint, d]) => (
            <button
              key={label}
              type="button"
              className={cx(
                "menu-item",
                `quick-${BUCKET_CLASS[dueBucket(d, today)]}`,
                d === selected && "checked",
              )}
              onClick={() => {
                onPick(d, timeStr);
                close();
              }}
            >
              <span className="menu-label">{label}</span>
              <span className="menu-hint">{hint}</span>
              {d === selected ? (
                <span className="menu-check">
                  <CheckSmallIcon />
                </span>
              ) : null}
            </button>
          ))}
          <button
            type="button"
            className="menu-item"
            onClick={() => {
              setStep("card");
              onCalendar?.();
            }}
          >
            <span className="menu-label">Pick a date and time</span>
          </button>
          <button
            type="button"
            className="menu-item"
            onClick={() => {
              setTimeText("");
              onPick(null, "");
              close();
            }}
          >
            <span className="menu-label">{emptyText}</span>
          </button>
        </div>
      ) : null}
      {step === "repeat" && repeat !== undefined ? (
        <div className="sched-quick">
          <button type="button" className="menu-item sched-back" onClick={() => setStep("card")}>
            <ChevronLeftIcon />
            <span className="menu-label">Back to date</span>
          </button>
          {repeat.items.map((item) => (
            <button
              key={item.value}
              type="button"
              className={cx("menu-item", item.value === repeat.selected && "checked")}
              onClick={() => {
                repeat.onSelect(item.value);
                setStep("card");
              }}
            >
              <span className="menu-label">{item.label}</span>
              {item.value === repeat.selected ? (
                <span className="menu-check">
                  <CheckSmallIcon />
                </span>
              ) : null}
            </button>
          ))}
        </div>
      ) : null}
      {step === "card" ? (
        <>
          {actions !== undefined ? (
            <div className="sched-actions">
              <button type="button" className="icon-btn" aria-label="Cancel" onClick={actions.onCancel}>
                <XIcon />
              </button>
              <button
                type="button"
                className="icon-btn sched-done"
                aria-label="Done"
                onClick={actions.onDone}
              >
                <CheckSmallIcon />
              </button>
            </div>
          ) : null}
          <div className="sched-cal">
            <div className="cal-nav">
              <button
                type="button"
                className={choosing ? "cal-title chooser open" : "cal-title chooser"}
                title="Choose month and year"
                onClick={() => setChoosing((c) => !c)}
              >
                {view.title}
                <ChevronDownSmallIcon />
              </button>
              <button
                type="button"
                className="icon-btn"
                title="Previous month"
                onClick={() => setMonth(shiftMonth(y, mo, -1))}
              >
                <ChevronLeftIcon />
              </button>
              <button
                type="button"
                className="icon-btn"
                title="Today"
                onClick={() => setMonth(yearMonth(today))}
              >
                <span className="cal-dot" />
              </button>
              <button
                type="button"
                className="icon-btn"
                title="Next month"
                onClick={() => setMonth(shiftMonth(y, mo, 1))}
              >
                <ChevronRightIcon />
              </button>
            </div>
            {choosing ? (
              // Month and year wheels, like iOS Calendar's; they stand in for
              // the day grid while open.
              <div ref={wheel} className="wheel">
                <div className="wheel-band" />
                {/* The twelve months, looping: wheel.ts places each row around the band. */}
                <div className="wheel-col loop" data-col="0">
                  {monthNames().map((name, i) => (
                    <div key={name} className={i + 1 === mo ? "wheel-item on" : "wheel-item"} data-i={i}>
                      {name}
                    </div>
                  ))}
                </div>
                <div className="wheel-col" data-col="1">
                  {wheelYears.map((year, i) => (
                    <div key={year} className={year === y ? "wheel-item on" : "wheel-item"} data-i={i}>
                      {year}
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div className="cal-grid">
                {view.weekdays.map((d, i) => (
                  // biome-ignore lint/suspicious/noArrayIndexKey: weekday letters repeat (T, S); their column is the key.
                  <span key={`h${i}`} className="cal-dow">
                    {d}
                  </span>
                ))}
                {view.weeks
                  .filter((week) => week.some((cell) => cell.inMonth))
                  .flat()
                  .map((cell) => {
                    const cls = ["cal-day"];
                    if (!cell.inMonth) cls.push("other");
                    if (cell.past) cls.push("past");
                    if (cell.today) cls.push("is-today");
                    // A date's fill follows its bucket; a deadline's stays the plain selected color.
                    if (cell.selected) cls.push("selected");
                    if (cell.selected && withTime)
                      cls.push(`sel-${BUCKET_CLASS[dueBucket(cell.date, today)]}`);
                    if (cell.repeat) cls.push("repeat-day");
                    return (
                      <button
                        key={cell.date}
                        type="button"
                        className={cls.join(" ")}
                        onClick={() => {
                          onPick(cell.date, timeStr);
                          // A date still has its Time and Repeat rows to offer.
                          if (!withTime && actions === undefined) close();
                        }}
                      >
                        {cell.day}
                      </button>
                    );
                  })}
              </div>
            )}
          </div>
          {withTime && touch ? (
            <TimeWheel
              time={timeStr}
              onChange={(t) => {
                setTimeText(t === "" ? "" : timeLabel(t));
                // A time without a date means today.
                onPick(t === "" ? selected : (selected ?? today), t);
              }}
            />
          ) : withTime ? (
            <div className="sched-time">
              <span className="sched-row-label">Time</span>
              <input
                className={timeBad ? "time-input bad" : "time-input"}
                placeholder="Add time, e.g. 5pm or 17:30"
                value={timeText}
                onChange={(e) => {
                  setTimeBad(false);
                  setTimeText(e.target.value);
                }}
                onBlur={commitTime}
                onKeyDown={(e) => {
                  if (e.key !== "Enter") return;
                  e.preventDefault();
                  e.stopPropagation();
                  if (commitTime()) close();
                }}
              />
              {time !== null ? (
                <button
                  type="button"
                  className="icon-btn"
                  title="Remove time"
                  onClick={() => {
                    setTimeText("");
                    onPick(selected, "");
                  }}
                >
                  <XSmallIcon />
                </button>
              ) : null}
            </div>
          ) : (
            <button
              type="button"
              className="menu-item sched-row"
              onClick={() => {
                onPick(null, "");
                close();
              }}
            >
              <span className="menu-label">{emptyText}</span>
            </button>
          )}
          {withTime && repeat !== undefined ? (
            <button type="button" className="menu-item sched-row" onClick={() => setStep("repeat")}>
              <span className="menu-label">Repeat</span>
              <span className="menu-hint">{repeat.text}</span>
            </button>
          ) : null}
        </>
      ) : null}
    </div>
  );
}
