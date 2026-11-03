// The streak in the task view: a popover like the field pickers', hanging from
// the streak row or the •••.
import { useRef } from "react";
import type { Task } from "../bindings";
import { usePlacedPopover } from "../editor/popover";
import { StreakPanel } from "../lists/StreakPanel";

export function StreakPopover({ task, onChanged }: { task: Task; onChanged: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  return (
    <div ref={ref} className="popover streak-pop">
      <StreakPanel task={task} onChanged={onChanged} />
    </div>
  );
}
