# Moving one occurrence of a repeating task (issue #8, part c)

Issue #8: moving a task that repeats every Wednesday to a Tuesday or Thursday changes the whole repeat instead of just that occurrence.

Before this change: a date pick sends the repeat choice again, and the preset choices ("Every week", "Every month") re-anchor on the new date, so the rule became "every Thu". Custom rules kept their rule, but completing an occurrence moved earlier (Wed → Tue) came back to that same Wednesday, and a streak counted the moved one as late.

## Decision (Noah, 2026-11-07)
A date pick on a repeating task always moves just this occurrence, like Todoist. The repeat changes only through the Repeat menu (or a typed repeat). The date shows the move: "Thu 12 Nov · moved from Wed 11 Nov".

With a streak going, moving first asks: a popup says the streak will reset, with Move and Cancel. Moving resets the current streak to 0 (the best stays), as changing the rule does.

## Design
- `Due.moved_from: Option<String>` (`YYYY-MM-DD`, `#[serde(default)]`): the occurrence the due date stands in for.
- `TaskUpdate` with the same rule and a new date: `moved_from` becomes the original occurrence (the old `moved_from`, or the old date when it's an occurrence). Moving onto a real occurrence (back to the original, or a week on) clears it. A new rule clears it.
- A move resets the streak (`streak_current`, `streak_last_period`, `last_completion`), keeping the best. `editor::moves_occurrence(before, after, existing)` says whether an edit is a move, so the UI can ask first when `streak_current > 0`.
- Completing: the occurrence completed is `moved_from` when set, and the next date is the first occurrence after the later of it and the due date. So Wed → Tue completes to next Wed, and Wed → Thu completes to next Wed. The same goes for repeating sub-tasks and for the repeat's end (`repeat_ends`).
- Undo: `LastCompletion.moved_from` (`#[serde(default)]`) puts it back.
- Editor (`changes`): while the repeat choice and typed rule are unchanged, the rule sent is the task's own (`existing`), never one re-anchored on the new date. `EditorValues.moved_from` comes from the task; `editor_view` adds "· moved from …" to the date chip while the date is off the rule.

- A repeating task now loads into the editor with its own rule ("keep"), not a preset: presets follow the date, so after a move "Every week" would have read as the new day. Presets that are the same rule as the task's are left out of the menu, so it has no duplicate row.
- UI: the task view's pickers save through `saveValues`; when `moves_occurrence` and `streak_current > 0`, `MovePrompt` asks (Cancel / Move) before saving.

Not done: list rows don't say a task was moved (only its date shows). A date typed into the task view's title moves the occurrence and resets the streak without asking (it saves on leaving the title, often while the view closes).
