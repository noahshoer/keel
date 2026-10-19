# SDD ledger — plan: docs/superpowers/plans/2026-10-19-quick-add-parser.md

## Pre-flight
- T1 parse -> T3 UI. T2 time_label -> T3. Consistent.

## Progress
- Task 1: Ruling: when stripping would leave an empty title (e.g. typing just "tomorrow"), nothing is applied and the text is the title — tightened from the plan's wording ("original text kept as title") so a lone date word doesn't silently also set a due. Cost if wrong: typing only "tomorrow" creates a task named "tomorrow" without a date.
- Task 1: Ruling: bare hours without am/pm ("at 5") are not treated as times (ambiguous). Cost if wrong: users must write 5pm/17:00.
Task 1: complete (commits 5e6261f..7834ace, tests: core 51/51)
Task 2: complete (commits 7834ace..b62560b, tests: core 53/53)
- Task 3: Ruling: quick add also applies in the "Add sub-task" editor (it's an add), never when editing. A bare /Section (no #project) resolves within the Inbox and takes the Inbox as project.
- Task 3: Note: dx 0.7.10 prints an "incompatible with dioxus 0.6.3" error line but the build completes and the wasm is correct (verified string present).
Task 3: complete (commits b62560b..eb2e4de, tests: workspace all green, wasm check 0 warnings)
