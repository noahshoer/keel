// Whether lists show sub-tasks under their parents, or only the parent with its
// "2 of 5" count (the default). A per-device preference, like the completion mark.
import { createContext } from "react";

/** This device's choice (localStorage `show-subtasks`); off by default. */
export function savedShowSubtasks(): boolean {
  try {
    return localStorage.getItem("show-subtasks") === "1";
  } catch {
    return false;
  }
}

/** Remembers this device's choice. */
export function saveShowSubtasks(on: boolean): void {
  try {
    localStorage.setItem("show-subtasks", on ? "1" : "0");
  } catch {
    // Private mode: the choice still applies for this session.
  }
}

/** The choice (App provides the saved one) and how Settings changes it. */
export const ShowSubtasksContext = createContext<{ show: boolean; setShow: (on: boolean) => void }>({
  show: false,
  setShow: () => {},
});
