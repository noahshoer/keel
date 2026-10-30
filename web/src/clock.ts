// The device clock, as the screens need it. This is about "now", not about
// tasks: anything that compares a task's date to today goes to keel-core
// (src/core) with the string localToday() returns.

const pad = (n: number) => String(n).padStart(2, "0");

/** Today's local calendar date as `YYYY-MM-DD` (what keel-core takes as `today`). */
export function localToday(now: Date = new Date()): string {
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

const WEEKDAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"] as const;
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"] as const;

/** The Today view's subtitle, "Thursday 29 Oct" (Dioxus `weekday` + `short_date`). */
export function daySubtitle(now: Date = new Date()): string {
  return `${WEEKDAYS[now.getDay()]} ${now.getDate()} ${MONTHS[now.getMonth()]}`;
}
