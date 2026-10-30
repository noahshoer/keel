// iOS raises the keyboard only for a focus made inside the user's tap, so a
// form opened by a tap renders synchronously and focuses its field in the
// same handler (not through autoFocus after a later render).
import type { RefObject } from "react";
import { flushSync } from "react-dom";

export function openAndFocus(open: () => void, field: RefObject<HTMLInputElement | null>): void {
  flushSync(open);
  field.current?.focus();
}
