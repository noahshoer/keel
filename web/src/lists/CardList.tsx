// Lists as cards (crates/ui/src/cards.rs): each top-level task is a card,
// and in dated views the sub-tasks that match sit inside their parent's card.
import { type ReactNode, useRef } from "react";
import type { Card, CardRow, Id, Label, Reminder } from "../bindings";
import { ChevronRightIcon, FlagFilledIcon } from "../shell/icons";
import { useTaskNav } from "../taskview/nav-context";
import { dueChip, FLAG_CLASS } from "./chips";
import { useFreshIds, useGrowIn } from "./presence";
import { INBOX_LOCATION, type RowLocation } from "./rowLayout";
import { SubtaskProgress } from "./SubtaskProgress";
import { TaskRow } from "./TaskRow";

/** A card's header when its task doesn't match the view: context only, no circle. */
function CardHead({ row, today }: { row: CardRow; today: string }) {
  const { task, progress } = row;
  const flag = FLAG_CLASS[task.priority];
  const due = dueChip(task, today);
  const nav = useTaskNav();
  // Dioxus opens just [id]; the chain open is the same here, as card heads are top-level.
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
        {progress !== null ? <SubtaskProgress progress={progress} /> : null}
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
  labels,
  reminders,
  onChanged,
}: {
  cards: Card[];
  today: string;
  /** Each task's project and section. */
  locations: Map<Id, RowLocation>;
  labels: Label[];
  reminders: Reminder[];
  onChanged: () => void;
}) {
  const where = (id: Id) => locations.get(id) ?? INBOX_LOCATION;
  // A card that wasn't here last time (a new task, one moved in) grows in.
  const fresh = useFreshIds(cards.map((c) => c.head.task.id));
  return cards.map((card) => (
    <CardBox key={card.head.task.id} fresh={fresh.has(card.head.task.id)}>
      {card.head_matches ? (
        <TaskRow
          row={card.head}
          today={today}
          location={where(card.head.task.id)}
          labels={labels}
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
          labels={labels}
          reminders={reminders}
          onChanged={onChanged}
        />
      ))}
    </CardBox>
  ));
}

/** A card's box; one that's `fresh` when it mounts opens from nothing. */
function CardBox({ fresh, children }: { fresh: boolean; children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  useGrowIn(ref, fresh);
  return (
    <div ref={ref} className="task-card">
      {children}
    </div>
  );
}
