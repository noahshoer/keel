// Which popover an editor has open: one at a time. The editor owns it and
// draws a transparent backdrop that closes it.
export type Pop =
  | "date"
  | "deadline"
  | "reminders"
  | "priority"
  | "repeat"
  | "location"
  | "labels"
  // Settings' reminder menus.
  | "offset"
  | "allday"
  | "summary"
  | null;

/** What every picker takes to open and close itself. */
export type PopProps = { open: Pop; setOpen: (p: Pop) => void };

export const cx = (...classes: Array<string | false | null | undefined>): string =>
  classes.filter(Boolean).join(" ");

/**
 * How a picker's button looks outside the editor's chip row: the task view's
 * field rows and dotted "+ …" pills (crates/ui/src/pickers.rs `Face`). The
 * popover is the same.
 */
export type Face = {
  /** The button's classes, in place of the chip's. */
  className: string;
  /** The button's text, in place of the chip's. */
  text: string;
  /** Shows the picker's icon before the text. */
  icon: boolean;
};
