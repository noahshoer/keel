// What the device says about itself, as the pickers need it.

type WeekInfo = { firstDay?: number } | undefined;
type LocaleWithWeek = Intl.Locale & { getWeekInfo?: () => WeekInfo; weekInfo?: WeekInfo };

/**
 * The locale's first day of the week, 1 (Monday) … 7 (Sunday): the calendar's
 * first column. Without week info it's Sunday; only Sunday and Saturday
 * change it from Monday (`pickers::provide_week_start` and `week_start`).
 */
export function weekStart(): number {
  try {
    const l = new Intl.Locale(navigator.language || "en-US") as LocaleWithWeek;
    const day = (l.getWeekInfo ? l.getWeekInfo() : l.weekInfo)?.firstDay ?? 7;
    return day === 7 || day === 6 ? day : 1;
  } catch {
    return 1;
  }
}

/** True on touch screens (phones), where pickers use native controls. */
export function isTouch(): boolean {
  return typeof matchMedia === "function" && matchMedia("(pointer: coarse)").matches;
}
