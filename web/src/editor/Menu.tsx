// A chip that opens a list of choices; the current one is checked
// (crates/ui/src/pickers.rs `Menu`): priority, repeat, project / section.
import { type ReactNode, useRef } from "react";
import { CheckSmallIcon, ChevronDownSmallIcon } from "../shell/icons";
import { cx, type Pop, type PopProps } from "./pop";
import { usePlacedPopover } from "./popover";

export type MenuItem = {
  value: string;
  label: string;
  icon?: ReactNode;
  iconClass?: string;
  indent?: boolean;
};

function MenuList({
  items,
  selected,
  onPick,
}: {
  items: MenuItem[];
  selected: string;
  onPick: (value: string) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  return (
    <div ref={ref} className="popover menu">
      {items.map((item) => {
        const checked = item.value === selected;
        return (
          <button
            key={item.value}
            type="button"
            className={cx("menu-item", item.indent && "indent", checked && "checked")}
            onClick={() => onPick(item.value)}
          >
            {item.icon !== undefined ? (
              <span className={cx("menu-icon", item.iconClass)}>{item.icon}</span>
            ) : null}
            <span className="menu-label">{item.label}</span>
            {checked ? (
              <span className="menu-check">
                <CheckSmallIcon />
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
  text,
  items,
  selected,
  open,
  setOpen,
  onSelect,
}: PopProps & {
  id: Exclude<Pop, null>;
  title: string;
  icon: ReactNode;
  iconClass?: string;
  text: string;
  items: MenuItem[];
  selected: string;
  onSelect: (value: string) => void;
}) {
  const isOpen = open === id;
  return (
    <div className="chip-wrap">
      <button
        type="button"
        className={cx("chip chip-btn", isOpen && "chip-open")}
        title={title}
        onClick={() => setOpen(isOpen ? null : id)}
      >
        <span className={cx("chip-icon", iconClass)}>{icon}</span>
        {text}
        <span className="chip-caret">
          <ChevronDownSmallIcon />
        </span>
      </button>
      {isOpen ? (
        <MenuList
          items={items}
          selected={selected}
          onPick={(value) => {
            onSelect(value);
            setOpen(null);
          }}
        />
      ) : null}
    </div>
  );
}
