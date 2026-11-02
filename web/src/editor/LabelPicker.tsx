// The Labels chip and its menu: tick labels, or type a new one and press
// Enter (it's created and ticked). crates/ui/src/editor.rs `LabelPicker`.
import { useRef, useState } from "react";
import type { Label } from "../bindings";
import { labelChoices, labelChosen, labelGroups } from "../core";
import { addLabel, setLabelColor } from "../ipc/actions";
import { CheckSmallIcon, TagSmallIcon } from "../shell/icons";
import { LabelColorPicker } from "./LabelColorPicker";
import { labelColorVar, labelTagColor } from "./labelColors";
import { cx, type Face, type PopProps } from "./pop";
import { usePlacedPopover } from "./popover";

function LabelMenu({
  labels,
  selected,
  onToggle,
  onChanged,
}: {
  labels: Label[];
  selected: string[];
  onToggle: (name: string) => void;
  onChanged: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  const [draft, setDraft] = useState("");
  const [coloring, setColoring] = useState<Label | null>(null);
  if (coloring !== null) {
    return (
      <div ref={ref} className="popover menu label-pop">
        <LabelColorPicker
          label={coloring}
          onBack={() => setColoring(null)}
          onPick={(color) => {
            setColoring(null);
            setLabelColor(coloring.id, color).then(onChanged, onChanged);
          }}
        />
      </div>
    );
  }
  const shown = labelChoices(labels, selected, draft);
  return (
    <div ref={ref} className="popover menu label-pop">
      <input
        className="label-new"
        placeholder="Find or add a label"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key !== "Enter") return;
          e.preventDefault();
          const name = draft.trim();
          if (!name) return;
          if (!labelChosen(selected, name)) onToggle(name);
          setDraft("");
          addLabel(name).then(onChanged, onChanged);
        }}
      />
      {labels.length === 0 ? <div className="label-empty">No labels yet</div> : null}
      {shown.map(({ label: l, on }) => {
        return (
          <div
            key={l.id}
            className={on ? "label-row on" : "label-row"}
            style={on ? { color: labelTagColor(l.color) } : undefined}
          >
            <label className="label-opt">
              <input type="checkbox" checked={on} onChange={() => onToggle(l.name)} />
              <span className="label-tag-icon" style={{ color: labelColorVar(l.color) }}>
                <TagSmallIcon />
              </span>
              {l.name}
            </label>
            <button
              type="button"
              className="label-color-btn"
              aria-label={`Color for ${l.name}`}
              onClick={() => setColoring(l)}
            >
              Color
            </button>
            {on ? (
              <span className="menu-check">
                <CheckSmallIcon />
              </span>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

export function LabelPicker({
  labels,
  selected,
  text,
  open,
  setOpen,
  onToggle,
  onChanged,
  face,
}: PopProps & {
  labels: Label[];
  /** The ticked names. */
  selected: string[];
  /** The chip's text. */
  text: string;
  onToggle: (name: string) => void;
  /** After a label was created (reload the lists). */
  onChanged: () => void;
  /** The task view's look for the button. */
  face?: Face;
}) {
  const isOpen = open === "labels";
  // The chip reads in the color of the label chosen last.
  const lastChosen = selected[selected.length - 1];
  const tint = lastChosen === undefined ? undefined : labelGroups([lastChosen], labels)[0];
  return (
    <div className="chip-wrap">
      <button
        type="button"
        className={cx(
          face?.className ?? "chip chip-btn",
          face === undefined && tint !== undefined && "pill-tinted",
          isOpen && "chip-open",
        )}
        title="Labels"
        style={tint !== undefined ? { color: labelColorVar(tint.color) } : undefined}
        onClick={() => setOpen(isOpen ? null : "labels")}
      >
        {face === undefined || face.icon ? (
          <span className="chip-icon pill-tag">
            <TagSmallIcon />
          </span>
        ) : null}
        {face?.text ?? text}
      </button>
      {isOpen ? (
        <LabelMenu labels={labels} selected={selected} onToggle={onToggle} onChanged={onChanged} />
      ) : null}
    </div>
  );
}
