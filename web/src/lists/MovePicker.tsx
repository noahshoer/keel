// The Move to… list a task's menu opens: every project with its sections
// nested, the task's place checked. Like the task card's project pill, whose
// items it shares. It opens against the row and closes the way the menu does.
import { type RefObject, useRef } from "react";
import { createPortal } from "react-dom";
import { locationChoices, locationPick } from "../core";
import { locationItems } from "../editor/items";
import { cx } from "../editor/pop";
import { CheckCircleIcon } from "../shell/icons";
import { useAnchoredPopup } from "../shell/useAnchoredPopup";
import type { Locations } from "./locations";

export function MovePicker({
  anchor,
  locations,
  current,
  onPick,
  close,
}: {
  anchor: RefObject<HTMLElement | null>;
  locations: Locations;
  /** Where the task is now: `project` or `project|section`. */
  current: string;
  onPick: (projectId: string, sectionId: string | null) => void;
  /** Must keep its identity. */
  close: () => void;
}) {
  const popup = useRef<HTMLDivElement>(null);
  useAnchoredPopup({ anchor, popup, close });
  const items = locationItems(locationChoices(locations.projects, locations.sections));
  return createPortal(
    // biome-ignore lint/a11y/useKeyWithClickEvents: only keeps a click in the portal from reaching the row.
    // biome-ignore lint/a11y/noStaticElementInteractions: see above.
    <div ref={popup} className="popover menu row-menu move-picker" onClick={(e) => e.stopPropagation()}>
      {items.map((item) => {
        const checked = item.value === current;
        return (
          <button
            key={item.value}
            type="button"
            className={cx("menu-item", item.indent && "indent", checked && "checked")}
            onClick={() => {
              close();
              const pick = locationPick(item.value);
              if (pick.type === "location") onPick(pick.projectId, pick.sectionId);
            }}
          >
            <span className={cx("menu-icon", item.iconClass)}>{item.icon}</span>
            <span className="menu-label">{item.label}</span>
            {checked ? (
              <span className="menu-check">
                <CheckCircleIcon />
              </span>
            ) : null}
          </button>
        );
      })}
    </div>,
    document.body,
  );
}
