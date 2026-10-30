# Dependencies

The web app installs code from npm, so every dependency is a supply-chain risk.
The guardrails below keep that risk small and visible.

## Policy

- Node is pinned in `.nvmrc`; pnpm is pinned with a hash in `packageManager`
  and installed through Corepack. No other package manager.
- `pnpm-workspace.yaml`:
  - `minimumReleaseAge: 10080`: a package must be at least 7 days old.
  - `strictDepBuilds: true`: no dependency lifecycle scripts run; every
    package with one is listed explicitly as ignored or allowed.
  - `trustPolicy: no-downgrade`: no provenance downgrades.
  - `blockExoticSubdeps: true`: no git or tarball sub-dependencies.
  - `saveExact: true`: exact versions, no ranges.
  - `engineStrict: true`: the Node engine range is enforced.
- Every direct dependency is in `deps-allowlist.json` with a reason, and
  `pnpm run check:deps` fails on a missing entry, a range version or an unused
  entry.
- Versions picked at install time are the newest one at least 7 days old.

## Adding a dependency

1. Real need: if it is a 20-line helper, write it instead.
2. Maintainers and download history look healthy.
3. npm provenance is present, or the package is named in "Provenance exceptions" below.
4. No install scripts.
5. Transitive count: compare `pnpm list --depth Infinity | wc -l` before and
   after, and inspect with `pnpm why`.
6. Licence is MIT, Apache, BSD or ISC.
7. Add an entry to `deps-allowlist.json` with the reason, then run
   `pnpm run check:deps`.

## Vetted dependencies

Checked on 2026-10-29 with read-only `npm view` queries. Provenance means the
registry serves `dist.attestations` for that exact version. None of the 13
direct dependencies declares an install script (`preinstall`, `install`,
`postinstall`).

| Package                | Version | Published  | Licence           | Provenance | Install script | Notes                                                                                                             |
| ---------------------- | ------- | ---------- | ----------------- | ---------- | -------------- | ----------------------------------------------------------------------------------------------------------------- |
| react                  | 19.3.1  | 2026-10-14 | MIT               | yes        | no             | 2 maintainers                                                                                                     |
| react-dom              | 19.3.1  | 2026-10-14 | MIT               | yes        | no             | Peer `react@^19.3.1`                                                                                              |
| typescript             | 7.0.2   | 2026-07-08 | Apache-2.0        | no         | no             | Microsoft; 7 maintainers. Full compiler with `tsc` (`tsc --version` prints 7.0.2). Published without attestations |
| vite                   | 8.3.5   | 2026-10-21 | MIT               | yes        | no             | Optional peers only                                                                                               |
| @vitejs/plugin-react   | 6.1.2   | 2026-10-05 | MIT               | yes        | no             | Peer `vite@^8.0.0` accepts Vite 8                                                                                 |
| vitest                 | 5.0.5   | 2026-10-15 | MIT               | yes        | no             | Peer `vite@^6.4 \|\| ^7 \|\| ^8`                                                                                  |
| @testing-library/react | 16.3.3  | 2026-08-27 | MIT               | yes        | no             | Peers: react 18/19, `@testing-library/dom@^10`                                                                    |
| @testing-library/dom   | 10.4.2  | 2026-09-13 | MIT               | yes        | no             |                                                                                                                   |
| jsdom                  | 30.1.3  | 2026-10-15 | MIT               | yes        | no             | `canvas` is an optional peer, not installed                                                                       |
| @biomejs/biome         | 2.6.2   | 2026-10-21 | MIT OR Apache-2.0 | yes        | no             | Platform binaries arrive as optional dependencies                                                                 |
| @types/react           | 19.3.0  | 2026-09-09 | MIT               | no         | no             | DefinitelyTyped; 1 listed maintainer (publishing bot)                                                             |
| @types/react-dom       | 19.3.0  | 2026-09-09 | MIT               | no         | no             | DefinitelyTyped                                                                                                   |
| @types/node            | 26.6.5  | 2026-10-09 | MIT               | no         | no             | DefinitelyTyped                                                                                                   |

Flags: no install scripts and no unusual licences. Four packages have no
provenance; they are covered by the exceptions below. No ownership changes
were investigated beyond the maintainer counts above.

## Provenance exceptions

A package without npm provenance is allowed only if it is listed here with its
publisher and a reason. Anything else without provenance is rejected.
- `typescript` (Microsoft): the compiler, a dev-only tool that never ships in
  the app bundle. It is published by Microsoft without registry attestations,
  and there is no alternative compiler.
- `@types/*` (DefinitelyTyped's publishing bot), covered as a group: type
  declarations only, no runtime code, dev-only.

## Guardrails verified

Checked on 2026-10-29 with pnpm 10.34.7 (Task 1 settings):
- `pnpm add typescript@7.1.0-dev.20261029.1` was refused: `ERR_PNPM_NO_MATURE_MATCHING_VERSION` (`minimumReleaseAge`).
- `pnpm add core-js` was refused: `ERR_PNPM_IGNORED_BUILDS` (`strictDepBuilds`).

Transitive count (`packages:` entries in `pnpm-lock.yaml`): 0 before Task 2
(empty lockfile), 151 after, for 13 direct dependencies.
