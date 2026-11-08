# One phrase vocabulary for quick add and filters

Date: 2026-11-08. Status: design, for a new GitHub issue (not yet filed) that comes before #10 (filter support in search) and #7 (filter and sort on list pages).

Mockup: https://claude.ai/artifact/VtYcUCsDiJdSkDBGZgDehB (private to Noah): quick add with new names, the save prompt, and search with an unknown name.

## Why

Keel has two parsers that read the same kind of text and share nothing:

- `keel_core::quickadd` reads task titles. It scans words, finds dates, times, repeats, priorities, `#project`, `/section`, `@label`, deadlines and streaks, and reports spans that the editor highlights. Backspace right after a highlight turns it back into plain text (`EditorValues::plain`).
- `keel_core::filter` reads saved filters. It splits on `& | ! ( ) ,` and expects each piece to be exactly one term from a short fixed list. Anything else is an error. It has its own small date parser and no spans.

Search (`keel_core::search`) is a third thing: whole-word text matching only.

#10 wants filter syntax in search, and #7 builds filter UI on top of the filter language. Both need a filter parser that accepts text, reports spans and understands the same phrases quick add does. Doing that once, first, means #10 becomes "search calls the filter parser", and #7 builds on a language that is already complete. Search must not care which terms exist: anything the language learns later works in search with no change to search.

Noah also wants every phrase defined in one place. Repeat phrases are the clearest case: there are many, and adding one should teach both quick add and filters.

## Goals

1. **One vocabulary.** The word-level matchers (date words and relative dates, times, repeat phrases, priorities, deadlines, `#` / `/` / `@` names) live in one module in `keel-core`. Quick add and the filter parser both call it. Adding a phrase there makes it work in both, with tests in one place.
2. **Filters accept text.** Plain words in a filter are a text term: the task contains them (same matching as search today). There is no separate "lenient mode"; text is simply part of the language. Saved filters may contain text too.
3. **Filters gain quick add's phrases.**
   - Relative and named dates as rolling date terms: `in 3 days`, `next friday`, `nov 30`, and the rest of quick add's date words. A bare date phrase means "due on that date", resolved against today each time the filter runs.
   - Repeat phrases as repeat terms: `every monday`, `every 2 weeks`, `every last workday`, and so on: tasks whose repeat matches.
   - Everything the filter language understands today keeps parsing (`today`, `overdue`, `p1`, `no date`, `next 7 days`, `due before:`, `deadline:`, `search:`, `#`, `##`, `/`, `@`, the operators, commas). What the date terms match changes for every filter, saved ones included (occurrences, below); Noah does not want the old matching kept.
   - **Every date term matches occurrences** (Noah). A date term matches any task with an occurrence on that date (or, for a range, in that range), not only its next due date: `today`, `tomorrow`, `next 7 days`, `due:`, `due before:`, `due after:` and every new date phrase. Relative dates resolve against today each time the filter runs. On a Sunday, `in 3 days` matches tasks due Wednesday, including a daily task whose next date is today; on Monday it matches Thursday. `overdue` is unchanged: only a task's next date can be overdue. Deadlines don't repeat, so deadline terms are unchanged.
4. **Spans everywhere.** The filter parser reports a span for every recognized term, with the same kind and color information quick add gives (`TitleSegment`: kind, priority level, due bucket, urgency). The search field and the filter editor (sidebar add form, `PageEdit` filter line) highlight like the task card.
5. **Backspace to plain, in filters too.** Backspace right after a highlighted term turns it into plain text, exactly like the title: `call mom tomorrow` with `tomorrow` backspaced searches for the word "tomorrow". Same mechanism and shape as `EditorValues::plain` (byte ranges, shifted as text changes, `quickadd::shift_plain`).
6. **Unknown names: an error in filters.** A marked term that names nothing (`#wrok`, `@nope`, `/nosuch`, `due:blursday`) is an error. The field shows it with a **red dotted underline** (not a wavy one), and an error line under the field: "No project named “wrok”." with a "Use #Work" fix when there is a close existing name. An unmarked word that isn't a keyword is never an error; it's text.
7. **Unknown names: new in quick add.** A `#name` or `@name` that doesn't exist yet is still taken as the task's project or label (labels already work this way; projects currently stay as text, and that changes). It is shown as new:
   - in the title, its highlight gets a **yellow dotted underline**;
   - its pill shows the name, **always filled yellow with a dotted outline**, overriding the project's or label's usual color.
8. **Confirm new names on save.** Saving a task that would create projects or labels first shows a prompt (mockup, artboard 2):
   - title "Create these with the task?" and the line "Untick any you don’t want. They stay as text in the title.";
   - one group for new projects and one for new labels, each with a header check and a count ("1 of 2");
   - each name is a row with a round check in Keel's style (filled accent circle with a white check when ticked, empty grey circle when not, the same look as the task check), next to its yellow pill;
   - every name starts ticked;
   - the header check is ticked when all are, shows a dash when some are (`aria-checked="mixed"`), and is empty when none are; tapping it unticks all when all are ticked, otherwise ticks all;
   - buttons "Back to editing" (returns to the card, nothing saved) and "Save" (creates the ticked names, then saves the task);
   - an unticked name is saved as plain text in the title, as if it had been backspaced.

## Non-goals

- Search behavior itself: filter syntax in the search field, and defaulting search to the current project or label, are #10. This work only makes the parser ready; the search field gets the parser in #10.
- Filter and sort UI on list pages (#7).
- Creating sections on the fly. An unknown `/section` in quick add behaves as today.
- Changing how highlights look, beyond the yellow and red states above.

## Anti-goals

- A second copy of any phrase list. If the filter parser needs to know a phrase, it gets it from the shared vocabulary.
- Breaking a saved filter's syntax. Every filter that parsed before must still parse (its matches may change as described above).
- Silent misreads. Quick add's rule stays: an `every …` phrase it can't fully read stays text as a whole. The same rule applies in filters.
- Data rules in TypeScript. Parsing, spans, closest-name suggestions, what counts as new and what an unticked name turns into are `keel-core`, exposed through wasm and IPC like the rest.

## Constraints

- `keel-core` stays pure and host-tested. TDD: failing test first.
- `Recurrence` variants are append-only. `Action` shapes don't change; creating a project or label on save uses the existing `ProjectAdd` / `LabelAdd` actions with temp ids, so the task can refer to the new project before it has a real id.
- Generated bindings (`web/src/bindings`) are regenerated with `scripts/check-bindings.sh` when types change.
- The existing quick-add tests (`quickadd.rs`, `editor.rs`) and filter tests (`filter.rs`) must keep passing; they are the compatibility suite. Filter tests about date matching for repeating tasks change with the occurrence rule. Two other expected exceptions: `filter.rs`'s `invalid_queries_error` (`foo`, `p5`, `3 dayz` become text), and the quick-add test that keeps an unknown `#x` as text (it becomes a new project).

## Decisions already made (Noah)

- The parser work is its own issue, done before #10 and #7.
- Hybrid language: plain text is valid, bare keywords highlight and can be backspaced to text, marked terms that don't resolve are errors in filters.
- Filters understand all of quick add's date and repeat phrases.
- Red dotted underline for errors in filters and search. Yellow dotted underline and yellow dotted pills for new names in quick add.
- New names are auto-taken in quick add and confirmed on save with the grouped, all-ticked prompt above.
- The filter editor gets the same highlighting as search.
- Plain text in a filter is written with quotes (`"tomorrow"`).
- Repeat phrases match by kind and interval, ignoring start and end.
- Every date term, old and new, matches occurrences; old matching is not kept.

## Calls made in writing this (check these)

- **Words next to each other are ANDed.** In a filter, `buy milk tomorrow` means text "buy milk" AND due tomorrow, without an `&`, the way quick add finds phrases inside a title. Operators still work and still bind as today (`,` then `|` then `&` then `!`).
- **Repeat terms match the kind and interval only** (Noah). `every 2 weeks` matches any task that repeats every two weeks, whatever its start date and whether or not it has an end (`until`, `for N days`). Weekdays named in the phrase must be among the task's (`every monday` matches "every mon, wed").
- **Plain text in a saved filter is written with quotes** (Noah). Backspacing a keyword in a filter field writes it as `"tomorrow"`, which the parser reads as the word. The field shows it as plain text. Quotes are part of the filter language, so search and saved filters read them the same way, and nothing new is stored.
- **How "has an occurrence on D" is computed:** a one-off task is due on D; a repeating task's next date is on or before D, its rule `contains(D)`, and D is not past its end. Built on `Recurrence::contains`, nothing stored. A task appears once in a result even when several of its occurrences fall in a range.
- **Unknown names in a saved filter still run.** Parsing stays free of names, as today: a saved `#OldProject` (renamed or deleted) still parses and that term matches nothing, so existing filters behave as before. Names are checked separately against the current projects, sections and labels, and that check drives the red underline and error line in the search field, the filter editors, and the filter page's header. A section name is known if any project has a section by that name.
- **Quick add's other phrases in filters:** a date with a time (`tomorrow at 5pm`) means due at that date and time; a deadline phrase (`{nov 30}`) means deadline on that date; `streak` means tasks with a streak on. `until …`, `for N days` and `starting …` stay text in a filter, highlighted as nothing.
- **The save prompt covers only names typed in the title.** A new label chosen from the label picker is an explicit choice and is still created without asking, as today. It appears wherever a typed title is applied: the add card, the sub-task card's "+ Add sub-task", and a retyped title in the task view (`applyTitle`), where it asks the way a streak reset already does. An unticked new project leaves the task in the project the card would otherwise use.
- **Numbers are not project names.** `#123` and `#1` (all digits or punctuation after the `#`) stay text, so "fix bug #123" doesn't offer a project called "123". The same rule applies to `@`.
- **"Use #Work" picks the closest existing name** by a simple edit distance, only when it's close (for example distance ≤ 2, or ≤ a third of the length), in `keel-core`. Used by the filter error line. Not offered in quick add, where the name is taken as new.
- **A new project name is one word** (`#wrok`), since an unknown multi-word name has no end. Known multi-word names keep matching as today.
- **The save prompt takes the card's place** while it's up, on desktop as on the phone.
- **Text terms reuse search's matching** (`search::matches`), so `buy milk` in a filter finds the same tasks search finds today.
- **`next 7 days` keeps its meaning** (a range). `next friday` is a single date. Where a phrase reads both ways, the filter's existing meaning wins, and a test pins it.

## Left to the builder

- Module layout: what the shared vocabulary module is called and how the two parsers drive it. One reasonable shape: a word scanner that returns `(span, Phrase)` where `Phrase` is a date, time, repeat, priority, name, deadline or keyword; quick add turns phrases into task fields, the filter turns them into `Term`s.
- How filter spans map onto `TitleSegment` (likely the same type, so `markClass` works unchanged) and the CSS for the yellow and red states.
- How the plain ranges travel for search and filters (IPC argument vs wasm call).
- How the error line and the "Use #Work" fix are wired in the sidebar form and `PageEdit`.
- The exact order of commands on save (create projects and labels, map temp ids, add the task).
