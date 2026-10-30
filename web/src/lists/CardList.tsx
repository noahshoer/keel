// Lists as cards (crates/ui/src/cards.rs): each top-level task is a card,
// and in dated views the sub-tasks that match sit inside their parent's card.
import type { Card, CardRow, Id, Reminder } from "../bindings";
import { ChevronRightIcon, FlagFilledIcon } from "../shell/icons";
import { useTaskNav } from "../taskview/nav-context";
import { dueChip, FLAG_CLASS } from "./chips";
import { SubtaskCount, TaskRow } from "./TaskRow";

/** A card's header when its task doesn't match the view: context only, no circle. */
function CardHead({ row, today }: { row: CardRow; today: string }) {
  const { task, progress } = row;
  const flag = FLAG_CLASS[task.priority];
  const due = dueChip(task, today);
  const nav = useTaskNav();
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: same markup as the Dioxus UI.
    // biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI.
    <div className="card-head" title="Open task" onClick={() => nav.open(task.id)}>
      {flag !== null ? (
        <span className={flag}>
          <FlagFilledIcon />
        </span>
      ) : null}
      <span className="pname">{task.content}</span>
      <span className="card-head-side">
        {due !== null ? <span className={due.cls}>{due.label}</span> : null}
        {progress !== null ? <SubtaskCount progress={progress} /> : null}
        <span className="card-chev">
          <ChevronRightIcon />
        </span>
      </span>
    </div>
  );
}

export function CardList({
  cards,
  today,
  locations,
  reminders,
  onChanged,
}: {
  cards: Card[];
  today: string;
  /** Each task's "Project / Section", or `null` in views that don't name it. */
  locations: Map<Id, string> | null;
  reminders: Reminder[];
  onChanged: () => void;
}) {
  const where = (id: Id) => (locations === null ? null : (locations.get(id) ?? ""));
  return cards.map((card) => (
    <div key={card.head.task.id} className="task-card">
      {card.head_matches ? (
        <TaskRow
          row={card.head}
          today={today}
          location={where(card.head.task.id)}
          reminders={reminders}
          onChanged={onChanged}
        />
      ) : (
        <CardHead row={card.head} today={today} />
      )}
      {card.rows.map((r) => (
        <TaskRow
          key={r.task.id}
          row={r}
          today={today}
          location={where(r.task.id)}
          reminders={reminders}
          onChanged={onChanged}
        />
      ))}
    </div>
  ));
}
