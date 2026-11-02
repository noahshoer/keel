// Touch: the Time row and a wheel inside the date card. A native time field
// would close the keyboard, since it takes focus from the title.
import { useEffect, useRef, useState } from "react";
import { timeLabel } from "../core";
import { partsOf, type TimeParts, timeOf } from "./timeParts";
import { installYearWheel } from "./wheel";

const HOURS = Array.from({ length: 12 }, (_, i) => i + 1);
const MINUTES = Array.from({ length: 60 }, (_, i) => i);
const HALVES = ["AM", "PM"];

function now(): string {
  const d = new Date();
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

export function TimeWheel({ time, onChange }: { time: string; onChange: (time: string) => void }) {
  const [open, setOpen] = useState(false);
  const wheel = useRef<HTMLDivElement>(null);
  const latest = useRef(time);
  latest.current = time;
  // What the wheel shows, which isn't saved until it's moved or set.
  const shownParts = useRef<TimeParts | null>(null);
  const changed = useRef(onChange);
  changed.current = onChange;

  useEffect(() => {
    const el = wheel.current;
    if (!open || !el) return;
    const parts = partsOf(latest.current === "" ? now() : latest.current);
    shownParts.current = parts;
    const column = (name: string) => el.querySelector<HTMLElement>(`[data-col="${name}"]`) as HTMLElement;
    const settle = () => changed.current(timeOf(parts));
    const stops = [
      installYearWheel(column("hour"), parts.hour - 1, (i) => {
        parts.hour = i + 1;
        settle();
      }),
      installYearWheel(column("minute"), parts.minute, (i) => {
        parts.minute = i;
        settle();
      }),
      installYearWheel(column("half"), parts.pm ? 1 : 0, (i) => {
        parts.pm = i === 1;
        settle();
      }),
    ];
    el.scrollIntoView?.({ block: "nearest" });
    return () => {
      for (const stop of stops) stop();
    };
  }, [open]);

  return (
    <>
      <button type="button" className="menu-item sched-row" onClick={() => setOpen(!open)}>
        <span className="menu-label">Time</span>
        <span className="menu-hint">{time === "" ? "Add time" : timeLabel(time)}</span>
      </button>
      {open ? (
        <div className="time-wheel">
          <div ref={wheel} className="wheel">
            <div className="wheel-band" />
            <div className="wheel-col" data-col="hour">
              {HOURS.map((h, i) => (
                <div key={h} className="wheel-item" data-i={i}>
                  {h}
                </div>
              ))}
            </div>
            <div className="wheel-col" data-col="minute">
              {MINUTES.map((m, i) => (
                <div key={m} className="wheel-item" data-i={i}>
                  {String(m).padStart(2, "0")}
                </div>
              ))}
            </div>
            <div className="wheel-col" data-col="half">
              {HALVES.map((h, i) => (
                <div key={h} className="wheel-item" data-i={i}>
                  {h}
                </div>
              ))}
            </div>
          </div>
          {time === "" ? (
            <button
              type="button"
              className="menu-item sched-row"
              onClick={() => {
                if (shownParts.current !== null) onChange(timeOf(shownParts.current));
              }}
            >
              <span className="menu-label">Set time</span>
            </button>
          ) : (
            <button
              type="button"
              className="menu-item sched-row"
              onClick={() => {
                onChange("");
                setOpen(false);
              }}
            >
              <span className="menu-label">Remove time</span>
            </button>
          )}
        </div>
      ) : null}
    </>
  );
}
