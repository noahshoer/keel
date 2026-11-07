# Typed repeats in the Repeat menu (issue #8, part b)

Issue #8: the task view's Repeat pill is hard to change a repeat with, since it only offers presets. Noah chose (2026-11-07): a text field heads the Repeat menu, everywhere that menu appears (task view row and pill, the date card's Repeat step, the add card's pill).

## Decisions
- Core (`keel_core::editor`): `read_repeat(text, date, today) -> Option<RepeatReading>` reads a repeat typed on its own (`quickadd::parse_repeat`): the phrase plus an optional start ("starting nov 1"), end ("for 10 days", "until dec 31") and time; nothing else (no title words, labels, priority…). "every" may be left off ("3 tuesdays"). It anchors on the task's date, or today when that's empty or past.
- A new pick, `Pick::RepeatText { text }`, applies it: `repeat = "custom"`, `EditorValues.rule` holds the rule, and the date, time and end follow the reading. Text that isn't a repeat changes nothing. A preset afterwards clears the typed rule (and the end, unless it's the task's own rule).
- `editor_view` lists the typed rule as a "custom" option, so it's the checked one, and the chip reads it with its end.
- UI: `Menu` takes an optional `typed` field. As you type, a row under the field shows the reading (or that it isn't one); Enter or a tap on that row sets it. The date card's Repeat step gets the same field. The field isn't focused on open, so the keyboard comes up only when it's tapped.
