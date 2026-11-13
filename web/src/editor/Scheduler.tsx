// The date popover, in steps: quick choices, then a month grid (or month / year
// wheels) with Time and Repeat rows, then the repeat options. A deadline is the
// grid alone. keel-core builds the grid and reads typed times; this draws them.
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { Recurrence } from "../bindings";
import { dueBucket, monthNames, monthView, quickDates, timeLabel, typedTime, yearChoices } from "../core";
import { BUCKET_CLASS } from "../lists/chips";
import { isTouch } from "../shell/device";
import {
  CheckCircleIcon,
  CheckSmallIcon,
  ChevronDownSmallIcon,
  ChevronLeftIcon,
  XIcon,
  XSmallIcon,
} from "../shell/icons";
import { CalendarMonths } from "./CalendarMonths";
import { afterKeyboardMove, unrollWithCard } from "./keyboardAnchor";
import { weekStart } from "./locale";
import type { MenuItem } from "./Menu";
import { cx } from "./pop";
import { usePlacedPopover } from "./popover";
import { RepeatField, type TypedRepeat } from "./RepeatField";
import { TimeWheel } from "./TimeWheel";
import { installMonthWheel, installYearWheel } from "./wheel";

export type CalendarActions = { onDone: () => void; onCancel: () => void };

/** The repeat choices the date card offers: the same ones as the Repeat pill. */
export type RepeatChoice = {
  items: MenuItem[];
  selected: string;
  /** The Repeat row's text. */
  text: string;
  onSelect: (value: string) => void;
  /** A field above the choices for a typed repeat. */
  typed?: TypedRepeat | undefined;
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
  // The quick choices fading out while the calendar waits for the keyboard to set off.
  const [leaving, setLeaving] = useState(false);
  const [month, setMonth] = useState<[number, number]>(() => yearMonth(selected ?? today));
  const monthRef = useRef(month);
  monthRef.current = month;
  // The years the calendar scrolls through and the wheels offer.
  const [years] = useState(() => yearChoices(today, yearMonth(selected ?? today)[0]));
  // The month / year wheels, opened from the title.
  const [choosing, setChoosing] = useState(false);
  const wheel = useRef<HTMLDivElement>(null);
  const [timeText, setTimeText] = useState(() => (time === null ? "" : timeLabel(time)));
  const [timeBad, setTimeBad] = useState(false);
  const [touch] = useState(isTouch);
  const [start] = useState(weekStart);
  const timeStr = time ?? "";
  // The grid with its X and check and nothing else: the task card's deadline.
  const gridOnly = !withTime && actions !== undefined;
  // It sits above the keyboard, where the full-size weeks don't fit.
  const compact = gridOnly && touch;

  useEffect(() => {
    if (!choosing) return;
    const [y, mo] = monthRef.current;
    const col = wheel.current?.querySelectorAll<HTMLElement>(".wheel-col");
    const months = col?.[0];
    const yearsCol = col?.[1];
    const stops: Array<() => void> = [];
    if (months) stops.push(installMonthWheel(months, mo - 1, (m0) => setMonth(([yy]) => [yy, m0 + 1])));
    if (yearsCol) {
      stops.push(
        installYearWheel(yearsCol, Math.max(0, years.indexOf(y)), (i) => {
          const year = years[i];
          if (year !== undefined) setMonth(([, m]) => [year, m]);
        }),
      );
    }
    return () => {
      for (const stop of stops) stop();
    };
  }, [choosing, years]);

  // Built while the card drops (it lowered the keyboard): it unrolls with the
  // card, or else unfolds on its own (app.css `sched-card`).
  useLayoutEffect(() => {
    if (step === "card" && touch && ref.current) unrollWithCard(ref.current);
  }, [step, touch]);

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
      // `sched-card` once the quick choices give way to the calendar, so its own entrance runs (app.css).
      className={cx(
        "popover scheduler",
        step === "card" && "sched-card",
        leaving && "sched-leaving",
        compact && "scheduler-compact",
      )}
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
                  <CheckCircleIcon />
                </span>
              ) : null}
            </button>
          ))}
          <button
            type="button"
            className="menu-item"
            onClick={() => {
              onCalendar?.();
              // On touch the calendar lowers the keyboard (`onCalendar`): it's built once
              // the card has set off with the keyboard, not before, which held the card back.
              if (touch && onCalendar) {
                setLeaving(true);
                afterKeyboardMove(() => {
                  setLeaving(false);
                  setStep("card");
                });
              } else setStep("card");
            }}
          >
            <span className="menu-label">Pick a date and time</span>
          </button>
          {/* A repeat always has a date. */}
          {rule === null ? (
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
          ) : null}
        </div>
      ) : null}
      {step === "repeat" && repeat !== undefined ? (
        <div className="sched-quick">
          <button type="button" className="menu-item sched-back" onClick={() => setStep("card")}>
            <ChevronLeftIcon />
            <span className="menu-label">Back to date</span>
          </button>
          {repeat.typed !== undefined ? (
            <RepeatField typed={repeat.typed} onDone={() => setStep("card")} />
          ) : null}
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
                  <CheckCircleIcon />
                </span>
              ) : null}
            </button>
          ))}
        </div>
      ) : null}
      {step === "card" ? (
        <>
          <div className="sched-head">
            {actions !== undefined ? (
              <button type="button" className="icon-btn" aria-label="Cancel" onClick={actions.onCancel}>
                <XIcon />
              </button>
            ) : null}
            <button
              type="button"
              className={choosing ? "cal-title chooser open" : "cal-title chooser"}
              title="Choose month and year"
              onClick={() => setChoosing((c) => !c)}
            >
              {view.title}
              <ChevronDownSmallIcon />
            </button>
            {actions !== undefined ? (
              <button
                type="button"
                className="icon-btn sched-done"
                aria-label="Done"
                onClick={actions.onDone}
              >
                <CheckSmallIcon />
              </button>
            ) : null}
          </div>
          <div className="sched-cal">
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
                  {years.map((year, i) => (
                    <div key={year} className={year === y ? "wheel-item on" : "wheel-item"} data-i={i}>
                      {year}
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <CalendarMonths
                years={years}
                shown={month}
                weekdays={view.weekdays}
                touch={touch}
                onShown={(year, m) => setMonth([year, m])}
                start={start}
                today={today}
                selected={selected}
                rule={rule}
                bucketFill={withTime}
                compact={compact}
                onDay={(date) => {
                  // With no row to clear it, the chosen day clears itself.
                  if (gridOnly && date === selected) {
                    onPick(null, "");
                    return;
                  }
                  onPick(date, timeStr);
                  // A date still has its Time and Repeat rows to offer.
                  if (!withTime && actions === undefined) close();
                }}
              />
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
          ) : actions === undefined ? (
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
          ) : null}
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
