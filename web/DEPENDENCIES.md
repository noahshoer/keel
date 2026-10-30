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
3. npm provenance is present.
4. No install scripts.
5. Transitive count: compare `pnpm list --depth Infinity | wc -l` before and
   after, and inspect with `pnpm why`.
6. Licence is MIT, Apache, BSD or ISC.
7. Add an entry to `deps-allowlist.json` with the reason, then run
   `pnpm run check:deps`.
