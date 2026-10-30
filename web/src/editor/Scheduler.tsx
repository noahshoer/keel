// The date popover: quick picks, a month grid (or month / year wheels), and
// the time (crates/ui/src/pickers.rs `Scheduler`). keel-core builds the
// grid and reads typed times; this draws them.
import { type ReactNode, useEffect, useRef, useState } from "react";
import type { Recurrence } from "../bindings";
import {
  dueBucket,
  dueLabel,
  monthNames,
  monthView,
  nativeTime,
  quickDates,
  shiftMonth,
  timeLabel,
  typedTime,
  yearChoices,
} from "../core";
import { BUCKET_CLASS } from "../lists/chips";
import {
  ArrowRightIcon,
  CalendarSmallIcon,
  ChevronDownSmallIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  ClockSmallIcon,
  NoDateIcon,
  SofaIcon,
  SunIcon,
} from "../shell/icons";
import { isTouch, weekStart } from "./locale";
import { usePlacedPopover } from "./popover";
import { installMonthWheel, installYearWheel } from "./wheel";

const QUICK: Array<[ReactNode, string]> = [
  [<CalendarSmallIcon key="i" />, "q-today"],
  [<SunIcon key="i" />, "q-tomorrow"],
  [<SofaIcon key="i" />, "q-weekend"],
  [<ArrowRightIcon key="i" />, "q-next"],
];

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
  today,
  onPick,
  close,
}: {
  /** `YYYY-MM-DD` or `null`. */
  selected: string | null;
  /** `HH:MM` or `null`. */
  time: string | null;
  /** Show the time row (off for deadlines). */
  withTime?: boolean;
  /** The header when nothing is picked. */
  emptyText?: string;
  /** The repeat rule whose days are ringed. */
  rule: Recurrence | null;
  today: string;
  /** A date (`null` for none) and a time (`HH:MM` or ""). */
  onPick: (date: string | null, time: string) => void;
  close: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
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
  const header = selected === null ? emptyText : dueLabel(selected, today);

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
      <div className="sched-head">{header}</div>
      <div className="sched-quick">
        {quickDates(today).map(([label, hint, d], i) => {
          const [icon, cls] = QUICK[i] ?? [null, ""];
          return (
            <button
              key={label}
              type="button"
              className="menu-item"
              onClick={() => {
                onPick(d, timeStr);
                close();
              }}
            >
              <span className={`menu-icon ${cls}`}>{icon}</span>
              <span className="menu-label">{label}</span>
              <span className="menu-hint">{hint}</span>
            </button>
          );
        })}
        <button
          type="button"
          className="menu-item"
          onClick={() => {
            setTimeText("");
            onPick(null, "");
            close();
          }}
        >
          <span className="menu-icon q-none">
            <NoDateIcon />
          </span>
          <span className="menu-label">No date</span>
        </button>
      </div>
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
          <button type="button" className="icon-btn" title="Today" onClick={() => setMonth(yearMonth(today))}>
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
            {view.weeks.flat().map((cell) => {
              const cls = ["cal-day"];
              if (!cell.inMonth) cls.push("other");
              if (cell.past) cls.push("past");
              if (cell.today) cls.push("is-today");
              if (cell.selected) cls.push("selected", `sel-${BUCKET_CLASS[dueBucket(cell.date, today)]}`);
              if (cell.repeat) cls.push("repeat-day");
              return (
                <button
                  key={cell.date}
                  type="button"
                  className={cls.join(" ")}
                  onClick={() => {
                    onPick(cell.date, timeStr);
                    close();
                  }}
                >
                  {cell.day}
                </button>
              );
            })}
          </div>
        )}
      </div>
      {withTime ? (
        <div className="sched-time">
          <ClockSmallIcon />
          {touch ? (
            // Phones: the OS time wheel instead of typing. iOS shows no
            // placeholder, so an empty field gets a hint.
            <span className="time-native">
              {timeStr === "" ? <span className="time-hint">Add time</span> : null}
              <input
                type="time"
                className="time-input native"
                value={timeStr}
                onChange={(e) => {
                  const t = nativeTime(e.target.value);
                  if (t === undefined) return;
                  if (t === null) {
                    setTimeText("");
                    onPick(selected, "");
                  } else {
                    setTimeText(timeLabel(t));
                    // A time without a date means today.
                    onPick(selected ?? today, t);
                  }
                }}
              />
            </span>
          ) : (
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
          )}
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
              ×
            </button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
