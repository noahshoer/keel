// A label's color choices, in place of the labels list in its menu.
import type { Label } from "../bindings";
import { ChevronLeftIcon, TagSmallIcon } from "../shell/icons";
import { LABEL_COLORS, labelColorVar } from "./labelColors";

export function LabelColorPicker({
  label,
  onPick,
  onBack,
}: {
  label: Label;
  onPick: (color: string) => void;
  onBack: () => void;
}) {
  return (
    <div className="label-colors">
      <button type="button" className="menu-item label-back" onClick={onBack}>
        <ChevronLeftIcon />
        <span className="menu-label">Back to labels</span>
      </button>
      <div className="label-color-name">
        <span className="label-tag-icon" style={{ color: labelColorVar(label.color) }}>
          <TagSmallIcon />
        </span>
        {label.name}
      </div>
      <div className="label-swatches">
        {LABEL_COLORS.map((c) => (
          <button
            key={c.id}
            type="button"
            aria-label={c.name}
            title={c.name}
            className={c.id === label.color ? "label-swatch selected" : "label-swatch"}
            onClick={() => onPick(c.id)}
          >
            <span style={{ background: labelColorVar(c.id) }} />
          </button>
        ))}
      </div>
    </div>
  );
}
