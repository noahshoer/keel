// Menu items shared by the editor's chips and the task view's fields
// (crates/ui/src/editor.rs `priority_items`, `location_items`, `flag_class`).
import type { LocationChoice, LocationKind } from "../bindings";
import { FlagFilledIcon, HashSmallIcon, InboxIcon, SectionSmallIcon } from "../shell/icons";
import type { MenuItem } from "./Menu";

/** A priority's flag colour ("4" is no priority). */
export const flagClass = (priority: string) =>
  ["1", "2", "3"].includes(priority) ? `pri-${priority}` : "pri-4";

export const PRIORITY_ITEMS: MenuItem[] = ["1", "2", "3", "4"].map((p) => ({
  value: p,
  label: `Priority ${p}`,
  icon: <FlagFilledIcon />,
  iconClass: flagClass(p),
}));

const LOCATION: Record<LocationKind, { icon: React.ReactNode; iconClass: string; indent: boolean }> = {
  Inbox: { icon: <InboxIcon />, iconClass: "loc-inbox", indent: false },
  Project: { icon: <HashSmallIcon />, iconClass: "loc-project", indent: false },
  Section: { icon: <SectionSmallIcon />, iconClass: "loc-section", indent: true },
};

/** The project / section menu's items (`EditorView.locations`). */
export const locationItems = (choices: LocationChoice[]): MenuItem[] =>
  choices.map((c) => ({ value: c.value, label: c.label, ...LOCATION[c.kind] }));
