// A text field that grows with its text: a textarea over a mirror
// (crates/ui/src/task_view/view.rs `GrowingText`). Enter is a new line; leaving
// it commits.
import { useLayoutEffect, useRef } from "react";
import { typedText } from "./drafts";

export function GrowingText({
  className,
  value,
  placeholder,
  autoFocus = false,
  onInput,
  onCommit,
}: {
  className: "tv-title" | "tv-desc";
  value: string;
  placeholder: string;
  /** Focus on mount. A tap that opens the field renders it in `flushSync`, so this lands inside the tap and iOS raises the keyboard. */
  autoFocus?: boolean;
  onInput: (text: string) => void;
  onCommit: () => void;
}) {
  const field = useRef<HTMLTextAreaElement>(null);
  // Mount only, as Dioxus' `onmounted`.
  // biome-ignore lint/correctness/useExhaustiveDependencies: focuses once, when the field appears.
  useLayoutEffect(() => {
    if (autoFocus) field.current?.focus();
  }, []);
  return (
    <div className={`tv-grow ${className}`}>
      <div className="tv-grow-mirror" aria-hidden="true">
        {`${value}​`}
      </div>
      <textarea
        ref={field}
        rows={1}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onInput(typedText(e.target.value))}
        onBlur={onCommit}
      />
    </div>
  );
}
