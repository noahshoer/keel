// The streak in the task view: a popover like the field pickers', hanging from
// the streak row or the •••. On touch, a tracked streak's sits on the keyboard
// instead, with its goal focused. That one is on the page, not in the sheet,
// whose expanding would carry it down first; `--kb` and its pace come from
// `gestures/sheet.ts`.
import { type RefObject, useRef } from "react";
import { createPortal } from "react-dom";
import type { Task } from "../bindings";
import { usePlacedPopover } from "../editor/popover";
import { StreakPanel } from "../lists/StreakPanel";

type Props = { task: Task; onChanged: () => void };

export function StreakPopover({ task, onChanged }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  return (
    <div ref={ref} className="popover streak-pop">
      <StreakPanel task={task} onChanged={onChanged} />
    </div>
  );
}

export function StreakOnKeyboard({
  task,
  onChanged,
  goal,
}: Props & { goal: RefObject<HTMLInputElement | null> }) {
  return createPortal(
    <div className="popover streak-pop keyboard-card">
      <StreakPanel task={task} onChanged={onChanged} goalField={goal} />
    </div>,
    document.body,
  );
}
