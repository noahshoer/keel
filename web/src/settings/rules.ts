// Settings' screen rules (crates/ui/src/settings.rs): the reminders switch's
// hint, the delivery note, an imported file's project name, and turning
// reminders on (which asks the system first).

/** The hint under "Task reminders". */
export function permissionHint(permission: string, enabled: boolean): string {
  if (permission === "denied") return "Blocked by the system";
  return enabled ? "On" : "Off";
}

/**
 * How reminders reach this device: the OS holds them on iPhone; the desktop
 * app shows them itself.
 */
export function deliveryNote(whenClosed: boolean): string {
  return whenClosed
    ? "Reminders arrive even when Keel is closed."
    : "Reminders appear while Keel is open on this computer.";
}

/** The project an imported file becomes: its name without the path or trailing `.csv`s (as Dioxus). */
export function projectNameOf(fileName: string): string {
  const base = fileName.split(/[/\\]/).pop() ?? fileName;
  return base.replace(/(\.csv)+$/, "").replace(/(\.CSV)+$/, "");
}

/**
 * Turning reminders on: asks the system unless already granted (a failed
 * request counts as denied). `granted` says whether to save them on.
 */
export async function turnOn(
  permission: string,
  request: () => Promise<string>,
): Promise<{ permission: string; granted: boolean }> {
  const state = permission === "granted" ? "granted" : await request().catch(() => "denied");
  return { permission: state, granted: state === "granted" };
}
