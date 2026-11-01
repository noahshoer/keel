// iOS: WebKit scrolls the whole page to reveal a focused field, moving the list
// under the task view or the task card; Swift holds the page still meanwhile
// (keel-ios PageScroll). Anyone can ask, and it's sent only when it changes.
import { invoke } from "../ipc/invoke";

let holders = 0;

/** Holds the page; returns the function that lets go. */
export function holdPage(): () => void {
  holders += 1;
  if (holders === 1) invoke("hold_page_scroll", { on: true }).catch(() => {});
  let held = true;
  return () => {
    if (!held) return;
    held = false;
    holders -= 1;
    if (holders === 0) invoke("hold_page_scroll", { on: false }).catch(() => {});
  };
}
