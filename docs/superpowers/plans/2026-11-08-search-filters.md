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
