// The Labels chip and its menu: tick labels, or type a new one and press
// Enter (it's created and ticked). crates/ui/src/editor.rs `LabelPicker`.
import { useRef, useState } from "react";
import type { Label } from "../bindings";
import { addLabel } from "../ipc/actions";
import { TagSmallIcon } from "../shell/icons";
import { cx, type PopProps } from "./pop";
import { usePlacedPopover } from "./popover";

const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

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
  return (
    <div ref={ref} className="popover menu label-pop">
      {labels.length === 0 ? <div className="label-empty">No labels yet</div> : null}
      {labels.map((l) => (
        <label key={l.id} className="label-opt">
          <input
            type="checkbox"
            checked={selected.some((n) => same(n, l.name))}
            onChange={() => onToggle(l.name)}
          />
          <TagSmallIcon />
          {l.name}
        </label>
      ))}
      <input
        className="label-new"
        placeholder="New label, then Enter"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key !== "Enter") return;
          e.preventDefault();
          const name = draft.trim();
          if (!name) return;
          if (!selected.some((n) => same(n, name))) onToggle(name);
          setDraft("");
          addLabel(name).then(onChanged, onChanged);
        }}
      />
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
}: PopProps & {
  labels: Label[];
  /** The ticked names. */
  selected: string[];
  /** The chip's text. */
  text: string;
  onToggle: (name: string) => void;
  /** After a label was created (reload the lists). */
  onChanged: () => void;
}) {
  const isOpen = open === "labels";
  return (
    <div className="chip-wrap">
      <button
        type="button"
        className={cx("chip chip-btn", isOpen && "chip-open")}
        title="Labels"
        onClick={() => setOpen(isOpen ? null : "labels")}
      >
        <span className="chip-icon">
          <TagSmallIcon />
        </span>
        {text}
      </button>
      {isOpen ? (
        <LabelMenu labels={labels} selected={selected} onToggle={onToggle} onChanged={onChanged} />
      ) : null}
    </div>
  );
}
