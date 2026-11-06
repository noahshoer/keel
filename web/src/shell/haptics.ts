// Vibrations (keel-ios Haptics.swift; nothing off iOS): `complete` is two quick
// taps, lower then higher, on the tap that completes a task; `press` is one deep
// thump when a long press registers. Never awaited, and a failure is ignored.
import type { HapticKind } from "../bindings/HapticKind";
import { invoke } from "../ipc/invoke";

export function haptic(kind: HapticKind): void {
  invoke("haptic", { kind }).catch(() => {});
}
