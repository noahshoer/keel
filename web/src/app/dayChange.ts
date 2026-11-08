// Tells the screens when the local date changes, so lists that depend on
// "today" reload at midnight. A timer aims at the next midnight; iOS freezes
// timers while the app is in the background, so coming back to the page
// (visibilitychange, focus) checks the date again too.
import { localToday } from "../clock";

function untilMidnight(now: Date): number {
  const next = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  // A beat past midnight, so the date has certainly rolled over.
  return next.getTime() - now.getTime() + 250;
}

/** Calls `onChange` each time the local date changes; returns the stop function. */
export function watchDayChange(onChange: () => void): () => void {
  let day = localToday();
  let timer: ReturnType<typeof setTimeout> | undefined;
  const check = () => {
    const now = localToday();
    if (now !== day) {
      day = now;
      onChange();
    }
  };
  const arm = () => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      check();
      arm();
    }, untilMidnight(new Date()));
  };
  const wake = () => {
    if (document.visibilityState === "hidden") return;
    check();
    arm();
  };
  arm();
  document.addEventListener("visibilitychange", wake);
  window.addEventListener("focus", wake);
  return () => {
    clearTimeout(timer);
    document.removeEventListener("visibilitychange", wake);
    window.removeEventListener("focus", wake);
  };
}
