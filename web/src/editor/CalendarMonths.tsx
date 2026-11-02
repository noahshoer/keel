// The calendar's days: one unbroken run of weeks in a vertical scroll that
// settles on the week of a month's 1st, like iOS Calendar. The month at the top
// is in full color, every other day grayed. keel-core builds the weeks; this
// draws the ones around the view.
import { type CSSProperties, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Recurrence } from "../bindings";
import { dueBucket, monthFirstWeeks, weekRows } from "../core";
import { BUCKET_CLASS } from "../lists/chips";
import { monthAt, weekWindow } from "./calendarScroll";

/** The view's height, in weeks: room for the longest month. */
const VISIBLE_WEEKS = 6;
/** Matches `.cal-day`'s size. */
const ROW_HEIGHT = 36;
const ROW_HEIGHT_TOUCH = 40;

export function CalendarMonths({
  years,
  shown,
  weekdays,
  touch,
  onShown,
  start,
  today,
  selected,
  rule,
  bucketFill,
  onDay,
}: {
  /** The years it scrolls through, in order. */
  years: number[];
  /** `[year, month]` at the top. */
  shown: [number, number];
  /** The weekday letters, in column order. */
  weekdays: string[];
  touch: boolean;
  /** When another month reaches the top. */
  onShown: (year: number, month: number) => void;
  start: number;
  today: string;
  selected: string | null;
  rule: Recurrence | null;
  /** A date's fill follows its bucket; a deadline's stays the plain selected color. */
  bucketFill: boolean;
  onDay: (date: string) => void;
}) {
  const firstYear = years[0] ?? shown[0];
  const lastYear = years[years.length - 1] ?? shown[0];
  const rowHeight = touch ? ROW_HEIGHT_TOUCH : ROW_HEIGHT;
  const firstWeeks = useMemo(() => monthFirstWeeks(firstYear, lastYear, start), [firstYear, lastYear, start]);
  const tops = useMemo(() => firstWeeks.map((w) => w * rowHeight), [firstWeeks, rowHeight]);
  // A last view's worth of weeks, so December can reach the top too.
  const total = (firstWeeks[firstWeeks.length - 1] ?? 0) + VISIBLE_WEEKS;
  const shownIndex = (shown[0] - firstYear) * 12 + shown[1] - 1;
  const opening = useRef(tops[shownIndex] ?? 0);
  const [[from, to], setDrawn] = useState(() => weekWindow(firstWeeks[shownIndex] ?? 0, total));
  const scroller = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    if (scroller.current) scroller.current.scrollTop = opening.current;
  }, []);

  const [shownYear, shownMonth] = shown;
  const weeks = useMemo(
    () => weekRows(firstYear, start, from, to - from, [shownYear, shownMonth], today, selected, rule),
    [firstYear, start, from, to, shownYear, shownMonth, today, selected, rule],
  );
  /** Week number to the `YYYY-MM` it starts. */
  const monthStarts = useMemo(() => {
    const starts = new Map<number, string>();
    firstWeeks.forEach((w, i) => {
      const year = firstYear + Math.floor(i / 12);
      starts.set(w, `${year}-${String((i % 12) + 1).padStart(2, "0")}`);
    });
    return starts;
  }, [firstWeeks, firstYear]);

  return (
    <>
      <div className="cal-grid">
        {weekdays.map((d, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: weekday letters repeat (T, S); their column is the key.
          <span key={`h${i}`} className="cal-dow">
            {d}
          </span>
        ))}
      </div>
      <div
        ref={scroller}
        className="cal-scroll"
        style={{ "--cal-row": `${rowHeight}px` } as CSSProperties}
        onScroll={(e) => {
          const top = e.currentTarget.scrollTop;
          const next = weekWindow(Math.floor(top / rowHeight), total);
          if (next[0] !== from || next[1] !== to) setDrawn(next);
          const at = monthAt(tops, top);
          if (at !== shownIndex) onShown(firstYear + Math.floor(at / 12), (at % 12) + 1);
        }}
      >
        <div className="cal-weeks" style={{ height: `${total * rowHeight}px` }}>
          {weeks.map((week, i) => {
            const number = from + i;
            const month = monthStarts.get(number);
            return (
              <div
                key={number}
                className={month === undefined ? "cal-grid cal-week" : "cal-grid cal-week cal-month-start"}
                data-month={month}
                style={{ top: `${number * rowHeight}px` }}
              >
                {week.map((cell) => {
                  const cls = ["cal-day"];
                  if (!cell.inMonth) cls.push("other");
                  if (cell.past) cls.push("past");
                  if (cell.today) cls.push("is-today");
                  if (cell.selected) cls.push("selected");
                  if (cell.selected && bucketFill) {
                    cls.push(`sel-${BUCKET_CLASS[dueBucket(cell.date, today)]}`);
                  }
                  if (cell.repeat) cls.push("repeat-day");
                  return (
                    <button
                      key={cell.date}
                      type="button"
                      className={cls.join(" ")}
                      onClick={() => onDay(cell.date)}
                    >
                      {cell.day}
                    </button>
                  );
                })}
              </div>
            );
          })}
        </div>
      </div>
    </>
  );
}
