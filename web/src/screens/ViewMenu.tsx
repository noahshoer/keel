// The ••• in a project, label or filter page's header: the same popup as a long
// press on its sidebar row. Edit happens on the page (PageEdit).
import { useRef, useState } from "react";
import { ActionPopup, type PopupStep } from "../shell/ActionPopup";
import { MoreIcon } from "../shell/icons";

export function ViewMenu({
  warning,
  onEdit,
  onDelete,
}: {
  warning: string;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const [step, setStep] = useState<PopupStep>("closed");
  const button = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button
        ref={button}
        type="button"
        className="icon-btn view-more"
        title="More"
        aria-label="More"
        onClick={() => setStep((s) => (s === "closed" ? "menu" : "closed"))}
      >
        <MoreIcon />
      </button>
      {step !== "closed" ? (
        <ActionPopup
          anchor={button}
          toggle={button}
          step={step}
          setStep={setStep}
          warning={warning}
          onEdit={() => {
            setStep("closed");
            onEdit();
          }}
          onDelete={onDelete}
        />
      ) : null}
    </>
  );
}
