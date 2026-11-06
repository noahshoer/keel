// Vibrations (keel-ios Haptics.swift; nothing off iOS): `complete` is one quick
// tap, on the tap that completes a task; `press` is one deep pop when a long
// press registers. Never awaited, and a failure is ignored.
import type { HapticKind } from "../bindings/HapticKind";
import { invoke } from "../ipc/invoke";

export function haptic(kind: HapticKind): void {
  invoke("haptic", { kind }).catch(() => {});
}
