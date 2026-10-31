// Completing a task (crates/ui/src/completion.rs): the circle fills with a
// Keel mark, then the row switches off like an old TV (squashes to a line,
// shrinks to a dot) before the task is actually completed. The mark is a
// per-device preference.
import { createContext } from "react";

/** How long the filled circle shows before the row switches off. */
export const FILL_MS = 350;
/** The switch-off animation (`.task.switch-off` in app.css). */
export const OFF_MS = 380;

export const COMPLETION_ICONS = ["wave", "pine", "reflection"] as const;
export type CompletionIcon = (typeof COMPLETION_ICONS)[number];

/** This device's choice (localStorage `completion-icon`, like the theme); the waterline by default. */
export function savedIcon(): CompletionIcon {
  try {
    const key = localStorage.getItem("completion-icon");
    return COMPLETION_ICONS.find((i) => i === key) ?? "wave";
  } catch {
    return "wave";
  }
}

/** Remembers this device's choice. */
export function saveIcon(icon: CompletionIcon): void {
  try {
    localStorage.setItem("completion-icon", icon);
  } catch {
    // Private mode: the choice still applies for this session.
  }
}

/** The name Settings shows for each mark. */
export function iconLabel(icon: CompletionIcon): string {
  if (icon === "pine") return "Pine";
  if (icon === "reflection") return "Pine + reflection";
  return "Waterline";
}

/** The mark every row's circle uses (App provides the saved one). */
export const CompletionIconContext = createContext<CompletionIcon>("wave");

/** 0 idle, 1 circle filled, 2 switching off. */
export type Phase = 0 | 1 | 2;

/** What a tap on the circle does in each phase. */
export function nextOnTap(phase: Phase): "start" | "cancel" | "ignore" {
  if (phase === 1) return "cancel";
  if (phase === 2) return "ignore";
  return "start";
}

export const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
