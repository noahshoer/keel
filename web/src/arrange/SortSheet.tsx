// The sort panel's contents: the heading fields (groups) and the order levels.
// Every edit calls onChange; the page saves it at once.
import { useMemo, useState } from "react";
import type { Arrangement, SortChoice, SortField } from "../bindings";
import { sortChoices } from "../core";
import { ChevronDownSmallIcon, PlusIcon, XSmallIcon } from "../shell/icons";
import {
  addGroup,
  addOrder,
  flipOrder,
  moveGroup,
  moveOrder,
  removeGroup,
  removeOrder,
  unusedGroups,
  unusedOrders,
} from "./sortEdit";

function Row({
  name,
  index,
  count,
  onMove,
  onRemove,
  children,
}: {
  name: string;
  index: number;
  count: number;
  onMove: (to: number) => void;
  onRemove: () => void;
  children?: React.ReactNode;
}) {
  return (
    <li className="sort-row">
      <span className="sort-name">{name}</span>
      {children}
      <button
        type="button"
        className="icon-btn sort-icon sort-up"
        aria-label={`Move ${name} up`}
        disabled={index === 0}
        onClick={() => onMove(index - 1)}
      >
        <ChevronDownSmallIcon />
      </button>
      <button
        type="button"
        className="icon-btn sort-icon"
        aria-label={`Move ${name} down`}
        disabled={index === count - 1}
        onClick={() => onMove(index + 1)}
      >
        <ChevronDownSmallIcon />
      </button>
      <button type="button" className="icon-btn sort-icon" aria-label={`Remove ${name}`} onClick={onRemove}>
        <XSmallIcon />
      </button>
    </li>
  );
}

function AddList({
  label,
  choices,
  onPick,
}: {
  label: string;
  choices: SortChoice[];
  onPick: (field: SortField) => void;
}) {
  const [open, setOpen] = useState(false);
  if (choices.length === 0) return null;
  return open ? (
    <div className="sort-choices">
      {choices.map((c) => (
        <button
          key={c.field}
          type="button"
          className="sort-choice"
          onClick={() => {
            setOpen(false);
            onPick(c.field);
          }}
        >
          {c.name}
        </button>
      ))}
    </div>
  ) : (
    <button type="button" className="sort-add" onClick={() => setOpen(true)}>
      <PlusIcon size={14} />
      {label}
    </button>
  );
}

export function SortSheet({
  arrangement,
  onChange,
}: {
  arrangement: Arrangement;
  onChange: (a: Arrangement) => void;
}) {
  // The fields and their labels never change.
  const choices = useMemo(sortChoices, []);
  const find = (field: SortField) => choices.find((c) => c.field === field);
  return (
    <div className="sort-sheet">
      <h3 className="sort-section">Group under headings</h3>
      <ul className="sort-list">
        {arrangement.groups.map((field, i) => (
          <Row
            key={field}
            name={find(field)?.name ?? field}
            index={i}
            count={arrangement.groups.length}
            onMove={(to) => onChange(moveGroup(arrangement, i, to))}
            onRemove={() => onChange(removeGroup(arrangement, i))}
          />
        ))}
      </ul>
      <AddList
        label="Add group"
        choices={unusedGroups(arrangement, choices)}
        onPick={(f) => onChange(addGroup(arrangement, f))}
      />
      <h3 className="sort-section">Then order tasks by</h3>
      <ul className="sort-list">
        {arrangement.order.map((level, i) => {
          const choice = find(level.field);
          return (
            <Row
              key={level.field}
              name={choice?.name ?? level.field}
              index={i}
              count={arrangement.order.length}
              onMove={(to) => onChange(moveOrder(arrangement, i, to))}
              onRemove={() => onChange(removeOrder(arrangement, i))}
            >
              <button type="button" className="sort-dir" onClick={() => onChange(flipOrder(arrangement, i))}>
                {level.reversed ? choice?.reversed_label : choice?.order_label}
              </button>
            </Row>
          );
        })}
      </ul>
      <AddList
        label="Add order"
        choices={unusedOrders(arrangement, choices)}
        onPick={(f) => onChange(addOrder(arrangement, f))}
      />
    </div>
  );
}
