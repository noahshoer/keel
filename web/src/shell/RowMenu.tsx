// A project, label or filter row's popup (and a section header's), which a long
// press opens (or a right click: gestures/long-press.ts). The row itself has no
// button. While the popup is open the row is lifted out of the dimmed page
// (`data-lifted`, an attribute since the row's component owns its `className`),
// so it's clear what the popup is for.
import { type RefObject, useEffect, useState } from "react";
import { ROW_PRESS } from "../gestures/long-press";
import { ActionPopup, type PopupStep } from "./ActionPopup";

export function RowMenu({
  row,
  warning,
  onEdit,
  onDelete,
}: {
  row: RefObject<HTMLElement | null>;
  warning: string;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const [step, setStep] = useState<PopupStep>("closed");
  useEffect(() => {
    const at = row.current;
    if (!at) return;
    const open = () => setStep("menu");
    at.addEventListener(ROW_PRESS, open);
    return () => at.removeEventListener(ROW_PRESS, open);
  }, [row]);
  const open = step !== "closed";
  useEffect(() => {
    const at = row.current;
    if (!at || !open) return;
    at.setAttribute("data-lifted", "");
    return () => at.removeAttribute("data-lifted");
  }, [row, open]);
  if (step === "closed") return null;
  return (
    <ActionPopup
      anchor={row}
      step={step}
      setStep={setStep}
      warning={warning}
      onEdit={onEdit}
      onDelete={onDelete}
    />
  );
}
