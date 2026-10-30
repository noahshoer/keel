// Which popover an editor has open: one at a time. The editor owns it and
// draws a transparent backdrop that closes it.
export type Pop = "date" | "deadline" | "reminders" | "priority" | "repeat" | "location" | "labels" | null;

/** What every picker takes to open and close itself. */
export type PopProps = { open: Pop; setOpen: (p: Pop) => void };

export const cx = (...classes: Array<string | false | null | undefined>): string =>
  classes.filter(Boolean).join(" ");
