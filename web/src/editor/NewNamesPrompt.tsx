// Saving a task whose title names projects or labels that don't exist yet
// asks first, in the card's place: each new name with a tick (all ticked to
// start), grouped by kind under a header check. Unticked names stay text in
// the title (keel-core `keepAsText`).
import { useState } from "react";
import type { NewNames } from "../bindings";
import { CheckIcon, MinusIcon } from "../shell/icons";
import { allTicked, countText, groupState, type Ticks, toggleGroup, unticked } from "./newNames";
import { cx } from "./pop";

const ARIA = { all: "true", some: "mixed", none: "false" } as const;

function Group({
  one,
  many,
  sign,
  ticks,
  onChange,
}: {
  one: string;
  many: string;
  /** `#` or `@`, before each name. */
  sign: string;
  ticks: Record<string, boolean>;
  onChange: (ticks: Record<string, boolean>) => void;
}) {
  const names = Object.keys(ticks);
  if (names.length === 0) return null;
  const state = groupState(ticks);
  return (
    <div className="nn-group">
      {/* biome-ignore lint/a11y/useSemanticElements: a tappable row drawn with Keel's round check; a header can be "mixed". */}
      <button
        type="button"
        role="checkbox"
        aria-checked={ARIA[state]}
        className="nn-head"
        onClick={() => onChange(toggleGroup(ticks))}
      >
        <span className={cx("check", state !== "none" && "done")}>
          {state === "some" ? <MinusIcon /> : <CheckIcon />}
        </span>
        <span className="nn-head-text">{names.length === 1 ? one : many}</span>
        <span className="nn-count">{countText(ticks)}</span>
      </button>
      {names.map((name) => (
        // biome-ignore lint/a11y/useSemanticElements: a tappable row drawn with Keel's round check.
        <button
          key={name}
          type="button"
          role="checkbox"
          aria-checked={ticks[name] ? "true" : "false"}
          className="nn-row"
          onClick={() => onChange({ ...ticks, [name]: !ticks[name] })}
        >
          <span className={cx("check", ticks[name] && "done")}>
            <CheckIcon />
          </span>
          <span className="nn-pill">
            {sign}
            {name}
          </span>
        </button>
      ))}
    </div>
  );
}

export function NewNamesPrompt({
  names,
  onBack,
  onSave,
}: {
  names: NewNames;
  /** Back to the card: nothing saved, the title as typed. */
  onBack: () => void;
  /** Creates the ticked names and saves; the unticked ones stay text. */
  onSave: (unticked: NewNames) => void;
}) {
  const [ticks, setTicks] = useState<Ticks>(() => allTicked(names));
  return (
    <div className="nn-prompt">
      <div className="nn-title">Create these with the task?</div>
      <div className="nn-line">Untick any you don’t want. They stay as text in the title.</div>
      <Group
        one="New project"
        many="New projects"
        sign="#"
        ticks={ticks.projects}
        onChange={(projects) => setTicks((t) => ({ ...t, projects }))}
      />
      <Group
        one="New label"
        many="New labels"
        sign="@"
        ticks={ticks.labels}
        onChange={(labels) => setTicks((t) => ({ ...t, labels }))}
      />
      <div className="nn-actions">
        <button type="button" className="btn btn-cancel" onClick={onBack}>
          Back to editing
        </button>
        <button type="button" className="btn btn-add" onClick={() => onSave(unticked(ticks))}>
          Save
        </button>
      </div>
    </div>
  );
}
