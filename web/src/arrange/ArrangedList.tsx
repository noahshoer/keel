// An arranged page with headings: keel-core's groups (`arrange`), each with its
// title line when it starts one, its sub-heading, and its cards. A task in two
// label groups is drawn in both, each copy in its own group's element.

import type { CSSProperties } from "react";
import type { ArrangedGroup, CardMode, Id, Label, Reminder, Task } from "../bindings";
import { cards } from "../core";
import { labelColorVar } from "../editor/labelColors";
import { CardList } from "../lists/CardList";
import type { RowLocation } from "../lists/rowLayout";
import { TagSmallIcon } from "../shell/icons";

export function ArrangedList({
  groups,
  all,
  mode,
  today,
  locations,
  labels,
  reminders,
  onChanged,
}: {
  groups: ArrangedGroup[];
  /** Every task, for the cards' sub-task counts. */
  all: Task[];
  mode: CardMode;
  today: string;
  locations: Map<Id, RowLocation>;
  labels: Label[];
  reminders: Reminder[];
  onChanged: () => void;
}) {
  return groups.map((group) => (
    <div key={group.key}>
      {group.starts_title && group.title !== null ? (
        <div className="section-title">
          {group.title}
          <span className="muted">{group.title_count}</span>
        </div>
      ) : null}
      {group.sub !== null ? (
        <div
          className="arrange-sub"
          style={
            group.sub.label_color != null
              ? ({ "--tint": labelColorVar(group.sub.label_color) } as CSSProperties)
              : undefined
          }
        >
          {group.sub.label_color != null ? <TagSmallIcon /> : null}
          {group.sub.text}
        </div>
      ) : null}
      <CardList
        cards={cards(group.tasks, all, mode, today)}
        today={today}
        locations={locations}
        labels={labels}
        reminders={reminders}
        onChanged={onChanged}
      />
    </div>
  ));
}
