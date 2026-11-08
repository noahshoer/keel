# Filter support in search (#10)

Search reads its query as the filter language and can be limited to the page it was opened from. Builds on the shared parser (`docs/superpowers/plans/2026-11-08-shared-parser.md`, "Note for #10").

## Decisions (Claude's calls, for Noah to veto)

- **Same language, same backend path.** `AppService::search_at(query, scope, today)` parses with `filter::parse` and matches with `filter::matches` (open and completed tasks). Plain words are one `Term::Text`, so word search is unchanged, except that `@name` is now the label term rather than a word.
- **Parse errors fall back to words.** A query that doesn't parse (`p1 &`) is searched as plain text, and the Search screen shows keel-core's problem under the box ("Searching for the words, not as a filter: …"). No empty list.
- **Comma groups are one list.** Unlike a filter page, search has one result list: a task matching any group appears once.
- **Ranking** stays title-first. `filter::text_words` gives the query's plain-text words (not those under `!`) to `search::rank`.
- **Scope is a predicate, not an `Expr`.** `SearchScope::{Project{id}, Label{name}}` (`keel_core::search`) is checked with every group, by id for projects (so same-named projects can't mix) and case-insensitively for labels. It isn't spliced into the query text, so the user's own `#`/`@` terms still parse as typed.
- **Scope comes from the page.** Search opened from a project or label page starts scoped to it (`searchScopeFor` in `web/src/app/view.ts`, taken each time Search opens from another page). A chip, "Searching in #Home ×", removes it. A deleted project drops the scope.
- **The boxes stay plain inputs.** Highlighting terms in the search box (a `FilterField`) was left out: it is a contenteditable, and the phone's search box has keyboard behavior tuned around a plain input. Possible follow-up.

## Tasks

- [x] Service tests, then `SearchScope`, `filter::text_words`, `search_at`, `SearchArgs.scope`
- [x] Bindings, mock backend (narrows its canned results by scope)
- [x] `searchScopeFor`, App state, SearchPane chip and problem line, tests
- [x] `cargo test`, `pnpm check`, browser preview at 375px (dark)

## Status

PR [#25](https://github.com/noahshoer/keel/pull/25), open (2026-11-08). Verified with `cargo test`, `pnpm check` and the browser preview (mock backend, 375px, dark). Not yet on the phone.

## Where this differs from the shared-parser plan's note for #10

- The scope is a predicate on each task (`SearchScope::contains`), not an `Expr::And` around the parsed groups.
- The search boxes are not `FilterField`s, so there is no term highlighting, red names or Backspace-to-quote in search.
- Comma groups are unioned into one list (the note left this open).

## To check on the phone

- The "Searching in" chip's size and tap target, and that removing it keeps the keyboard and query as they were.
- Opening Search by swipe (not the header button) from a project page starts scoped.

## Follow-ups (not started)

- Highlight terms in the search box (a `FilterField`, once its keyboard behavior is proven on the phone).
- Mention filter syntax in the empty Search state (`searchHint`; its text is pinned by tests today).
