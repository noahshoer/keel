// The device clock, as the screens need it. This is about "now", not about
// tasks: anything that compares a task's date to today goes to keel-core
// (src/core) with the string localToday() returns.

const pad = (n: number) => String(n).padStart(2, "0");

/** Today's local calendar date as `YYYY-MM-DD` (what keel-core takes as `today`). */
export function localToday(now: Date = new Date()): string {
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/** The local calendar date (`YYYY-MM-DD`) of an ISO timestamp; `null` when it doesn't parse. */
export function localDateOf(iso: string): string | null {
  const at = new Date(iso);
  return Number.isNaN(at.getTime()) ? null : localToday(at);
}
