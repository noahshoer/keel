// The Repeat menu's typed field (issue #8): any repeat quick add reads
// ("every 3 tuesdays until dec 31"). The row under it shows what keel-core
// reads, live; Enter or a tap on it sets the repeat. Not focused on open, so
// on the phone the keyboard waits for a tap on the field.
import { useState } from "react";
import { RepeatSmallIcon } from "../shell/icons";

export type TypedRepeat = {
  /** What the text reads as ("Every 3 weeks on Tue"), or null when it isn't a repeat. */
  read: (text: string) => string | null;
  onSubmit: (text: string) => void;
};

export function RepeatField({ typed, onDone }: { typed: TypedRepeat; onDone: () => void }) {
  const [text, setText] = useState("");
  const reading = text.trim() === "" ? null : typed.read(text);
  const submit = () => {
    if (reading === null) return;
    typed.onSubmit(text);
    onDone();
  };
  return (
    <div className="menu-typed">
      <label className="menu-typed-field">
        <span className="menu-icon menu-typed-icon">
          <RepeatSmallIcon />
        </span>
        <input
          type="text"
          className="menu-typed-input"
          aria-label="Type a repeat"
          placeholder="Type a repeat, e.g. every 3 tuesdays"
          autoComplete="off"
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          enterKeyHint="done"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              submit();
            }
          }}
        />
      </label>
      {text.trim() !== "" ? (
        <button type="button" className="menu-item menu-reading" disabled={reading === null} onClick={submit}>
          <span className="menu-label">{reading ?? "Not a repeat yet"}</span>
        </button>
      ) : null}
    </div>
  );
}
