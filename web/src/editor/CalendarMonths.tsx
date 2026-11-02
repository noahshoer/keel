// The calendar's days: months stacked in a vertical scroll that settles on the
// start of a month, like iOS Calendar. keel-core builds each month; this draws
// the few around the view.
import { type CSSProperties, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Recurrence } from "../bindings";
import { dueBucket, monthRows, monthView } from "../core";
import { BUCKET_CLASS } from "../lists/chips";
import { monthAt, monthTops, monthWindow } from "./calendarScroll";

/** The view's height: room for the longest month. */
const VISIBLE_ROWS = 6;
const MONTH_GAP = 12;
/** Matches `.cal-day`'s size. */
const ROW_HEIGHT = 36;
const ROW_HEIGHT_TOUCH = 40;

type DayProps = {
  start: number;
  today: string;
  selected: string | null;
  rule: Recurrence | null;
  /** A date's fill follows its bucket; a deadline's stays the plain selected color. */
  bucketFill: boolean;
  onDay: (date: string) => void;
};

function Month({
  year,
  month,
  top,
  start,
  today,
  selected,
  rule,
  bucketFill,
  onDay,
}: DayProps & { year: number; month: number; top: number }) {
  const view = useMemo(
    () => monthView(year, month, start, today, selected, rule),
    [year, month, start, today, selected, rule],
  );
  return (
    <div
      className="cal-grid cal-month"
      data-month={`${year}-${String(month).padStart(2, "0")}`}
      style={{ top: `${top}px` }}
    >
      {view.weeks
        .filter((week) => week.some((cell) => cell.inMonth))
        .flat()
        .map((cell) => {
          if (!cell.inMonth) return <span key={cell.date} />;
          const cls = ["cal-day"];
          if (cell.past) cls.push("past");
          if (cell.today) cls.push("is-today");
          if (cell.selected) cls.push("selected");
          if (cell.selected && bucketFill) cls.push(`sel-${BUCKET_CLASS[dueBucket(cell.date, today)]}`);
          if (cell.repeat) cls.push("repeat-day");
          return (
            <button key={cell.date} type="button" className={cls.join(" ")} onClick={() => onDay(cell.date)}>
              {cell.day}
            </button>
          );
        })}
    </div>
  );
}

export function CalendarMonths({
  years,
  shown,
  weekdays,
  touch,
  onShown,
  ...day
}: DayProps & {
  /** The years it scrolls through, in order. */
  years: number[];
  /** `[year, month]` to open on. */
  shown: [number, number];
  /** The weekday letters, in column order. */
  weekdays: string[];
  touch: boolean;
  /** When another month reaches the top. */
  onShown: (year: number, month: number) => void;
}) {
  const firstYear = years[0] ?? shown[0];
  const lastYear = years[years.length - 1] ?? shown[0];
  const rowHeight = touch ? ROW_HEIGHT_TOUCH : ROW_HEIGHT;
  const viewHeight = rowHeight * VISIBLE_ROWS;
  const tops = useMemo(
    () => monthTops(monthRows(firstYear, lastYear, day.start), rowHeight, MONTH_GAP),
    [firstYear, lastYear, day.start, rowHeight],
  );
  const [index, setIndex] = useState(() => (shown[0] - firstYear) * 12 + shown[1] - 1);
  const scroller = useRef<HTMLDivElement>(null);
  const opening = useRef(tops[index] ?? 0);

  useLayoutEffect(() => {
    if (scroller.current) scroller.current.scrollTop = opening.current;
  }, []);

  const [from, to] = monthWindow(index, tops.length);
  const months: number[] = [];
  for (let i = from; i <= to; i += 1) months.push(i);

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
          const at = monthAt(tops, e.currentTarget.scrollTop);
          if (at === index) return;
          setIndex(at);
          onShown(firstYear + Math.floor(at / 12), (at % 12) + 1);
        }}
      >
        {/* A last view's worth of room, so December can reach the top too. */}
        <div className="cal-months" style={{ height: `${(tops[tops.length - 1] ?? 0) + viewHeight}px` }}>
          {months.map((i) => (
            <Month
              key={i}
              year={firstYear + Math.floor(i / 12)}
              month={(i % 12) + 1}
              top={tops[i] ?? 0}
              {...day}
            />
          ))}
        </div>
      </div>
    </>
  );
}
