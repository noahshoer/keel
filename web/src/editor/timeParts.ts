// The time wheel's columns: hours 1 to 12, minutes, AM or PM, as `timeLabel` shows them.
export type TimeParts = { hour: number; minute: number; pm: boolean };

/** The columns for an `HH:MM` time. */
export function partsOf(time: string): TimeParts {
  const h = Number(time.slice(0, 2));
  return { hour: h % 12 === 0 ? 12 : h % 12, minute: Number(time.slice(3, 5)), pm: h >= 12 };
}

/** The `HH:MM` time the columns show. */
export function timeOf({ hour, minute, pm }: TimeParts): string {
  const h = (hour % 12) + (pm ? 12 : 0);
  return `${String(h).padStart(2, "0")}:${String(minute).padStart(2, "0")}`;
}
