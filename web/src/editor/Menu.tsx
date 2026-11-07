// A chip that opens a list of choices; the current one is checked
// (crates/ui/src/pickers.rs `Menu`): priority, repeat, project / section.
import { type ReactNode, useRef } from "react";
import { CheckCircleIcon, ChevronDownSmallIcon } from "../shell/icons";
import { cx, type Face, type Pop, type PopProps } from "./pop";
import { usePlacedPopover } from "./popover";
import { RepeatField, type TypedRepeat } from "./RepeatField";

export type MenuItem = {
  value: string;
  label: string;
  icon?: ReactNode;
  iconClass?: string;
  indent?: boolean;
  /** Added to the row while it's the chosen one. */
  checkedClass?: string | undefined;
};

function MenuList({
  items,
  selected,
  onPick,
  typed,
  onTyped,
}: {
  items: MenuItem[];
  selected: string;
  onPick: (value: string) => void;
  typed?: TypedRepeat | undefined;
  onTyped: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  return (
    <div ref={ref} className="popover menu">
      {typed !== undefined ? <RepeatField typed={typed} onDone={onTyped} /> : null}
      {items.map((item) => {
        const checked = item.value === selected;
        return (
          <button
            key={item.value}
            type="button"
            className={cx(
              "menu-item",
              item.indent && "indent",
              checked && "checked",
              checked && item.checkedClass,
            )}
            onClick={() => onPick(item.value)}
          >
            {item.icon !== undefined ? (
              <span className={cx("menu-icon", item.iconClass)}>{item.icon}</span>
            ) : null}
            <span className="menu-label">{item.label}</span>
            {checked ? (
              <span className="menu-check">
                <CheckCircleIcon />
              </span>
            ) : null}
          </button>
        );
      })}
    </div>
  );
}

export function Menu({
  id,
  title,
  icon,
  iconClass,
  chipClass,
  text,
  items,
  selected,
  open,
  setOpen,
  onSelect,
  face,
  typed,
}: PopProps & {
  id: Exclude<Pop, null>;
  title: string;
  icon: ReactNode;
  iconClass?: string;
  /** Added to the chip's own classes. */
  chipClass?: string;
  text: string;
  items: MenuItem[];
  selected: string;
  onSelect: (value: string) => void;
  /** The task view's look for the button. */
  face?: Face;
  /** A field above the choices for a typed repeat. */
  typed?: TypedRepeat | undefined;
}) {
  const isOpen = open === id;
  return (
    <div className="chip-wrap">
      <button
        type="button"
        className={cx(face?.className ?? cx("chip chip-btn", chipClass), isOpen && "chip-open")}
        title={title}
        onClick={() => setOpen(isOpen ? null : id)}
      >
        {face === undefined || face.icon ? <span className={cx("chip-icon", iconClass)}>{icon}</span> : null}
        {face === undefined ? (
          <>
            {text}
            <span className="chip-caret">
              <ChevronDownSmallIcon />
            </span>
          </>
        ) : (
          face.text
        )}
      </button>
      {isOpen ? (
        <MenuList
          items={items}
          selected={selected}
          onPick={(value) => {
            onSelect(value);
            setOpen(null);
          }}
          typed={typed}
          onTyped={() => setOpen(null)}
        />
      ) : null}
    </div>
  );
}
