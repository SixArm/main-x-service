# Svelte front-end stack — Lily package distribution

How the sixteen SvelteKit front-ends consume the Lily Design System's
Svelte packages, and why. This is a design decision document
(`tasks.md` WEB-2) — it fixes the answer, so no front-end re-litigates
it per PR.

## 1. The problem

Every front-end has always depended on the six Lily packages it needs
(`lily-design-system-svelte-headless`, `-theme-picker`,
`-locale-picker`, `-text-size-picker`, `-share-picker`,
`-picker-bar`) via a `file:` path onto a **sibling checkout outside
this repository**:

```
"lily-design-system-svelte-theme-picker": "file:../../../../lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers/lily-design-system-svelte-theme-picker"
```

That is fine for local development against a live, in-progress Lily
change, and it is why the pattern was adopted. It is fatal for CI: a
clean runner has no `~/git/lilydesignsystem/lily-design-system`
checkout, so `pnpm install` cannot resolve the dependency at all. This
is the reason no front-end CI stage has ever existed (`tasks.md`
WEB-2) — there was nothing for `pnpm install` to succeed against.

## 2. The options considered

- **Publish to the npm registry, depend on a semver range.** Simplest
  and most conventional; loses nothing once every package is
  published, since the published tarball already carries a built
  `dist/` (each package's `files` field lists `dist`).
- **A git-URL dependency pinned to a commit.** Avoids a publish step,
  but the Lily repo's `.gitignore` excludes `dist/` at both the repo
  root and the `lily-design-system-svelte-helpers/` level — confirmed
  by `git ls-files`, nothing under any package's `dist/` is tracked.
  A git dependency would therefore need to run its own build during
  `pnpm install` (a `prepare`/`postinstall` step compiling Svelte +
  TypeScript, for six packages, sixteen times over), which is real
  extra CI weight and a second build toolchain for something a publish
  already solves.
- **Vendor a copy into this repo.** Makes CI self-contained with no
  external repo dependency at all, at the cost of a sync process to
  keep sixteen vendored copies from drifting, and reintroducing
  exactly the kind of duplicated-copy drift the family has
  deliberately avoided elsewhere (`entity-ref`, `integrity-mac` are
  both **not** copied per consumer for this reason).

## 3. Decision: registry versions

**Publish, and depend on the published semver version.** This is not
hypothetical — checked 2026-09-16, every package this family uses is
**already published**, and the published version matches the sibling
checkout's current version exactly:

| Package | Local checkout | npm registry |
|---|---|---|
| `lily-design-system-svelte-headless` | 0.3.1 | 0.3.1 |
| `lily-design-system-svelte-theme-picker` | 0.1.1 | 0.1.1 |
| `lily-design-system-svelte-locale-picker` | 0.1.1 | 0.1.1 |
| `lily-design-system-svelte-text-size-picker` | 0.1.1 | 0.1.1 |
| `lily-design-system-svelte-share-picker` | 0.1.1 | 0.1.1 |
| `lily-design-system-svelte-picker-bar` | 0.1.0 | 0.1.0 |

So "publish and take a registry version" needs no new publishing work
today — only a `package.json` change, in every front-end, from a
`file:` path to a `^`-prefixed registry version:

```jsonc
"lily-design-system-svelte-headless": "^0.3.1",
"lily-design-system-svelte-locale-picker": "^0.1.1",
"lily-design-system-svelte-picker-bar": "^0.1.0",
"lily-design-system-svelte-share-picker": "^0.1.1",
"lily-design-system-svelte-text-size-picker": "^0.1.1",
"lily-design-system-svelte-theme-picker": "^0.1.1",
```

`pnpm install --frozen-lockfile` then succeeds on a clean runner with
no sibling checkout — the property WEB-2's CI stage needs.

## 4. The tradeoff, stated plainly

A Lily change in the sibling checkout no longer reaches a front-end's
`pnpm install` until it is **version-bumped and published**. This is
a real, accepted cost, not an oversight: it is the same publish-then-
consume relationship every other published dependency in this family
already has (`entity-ref`, `authentication-verifier`), and the
alternative (`file:`) is the thing that made front-end CI impossible
in the first place.

For local development against an **in-flight, unpublished** Lily
change, use `pnpm link` rather than a committed dependency change —
it is per-developer, touches no `package.json` or lockfile line, and
is exactly what `link` exists for:

```sh
cd person/person-front-end-with-svelte
pnpm link ../../../../lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers/lily-design-system-svelte-theme-picker
# … work against the live checkout …
pnpm unlink lily-design-system-svelte-theme-picker   # restore the registry version
```

## 5. Rollout

1. This doc.
2. Every front-end's `package.json`: six `file:` deps → six `^`-pinned
   registry versions; regenerate `pnpm-lock.yaml`
   (`pnpm install`); confirm `pnpm run check` / `pnpm test` /
   `pnpm run build` are unchanged (a dependency-resolution change, not
   a behavioural one — the published tarball and the local checkout
   are byte-identical in content, only the resolution path differs).
3. `scripts/ci-front-ends.sh` + the `front-end` CI job in both
   `ci.yml` and `.woodpecker.yml` (`tasks.md` WEB-2), which this
   change is the prerequisite for.

## 6. CI's pnpm version is pinned exact, not a floating major

`pnpm/action-setup` / `corepack prepare` name `11.0.8` exactly, not a
floating `11`. A newer 11.x enforces a `minimumReleaseAge` supply-chain
policy — `pnpm install --frozen-lockfile` refuses a lockfile entry
published within roughly the last 24 hours — which rejected
`lily-design-system-svelte-picker-bar@0.1.0` on the very PR that first
enrolled it, published only hours earlier. The policy itself is
reasonable (it defends against a just-compromised or typosquatted
package being picked up immediately); the fix is a stable, tested pnpm
version rather than reaching for `--trust-lockfile` or disabling the
policy outright to work around a default this repo has not evaluated.
Bump the pin deliberately, not incidentally via a floating major.

## 7. Open question

**Keeping registry versions current.** Nothing today automatically
bumps a front-end's Lily version when the packages publish a new
release — a front-end simply keeps using whatever `^`-range it last
resolved to until someone runs `pnpm update` for that package. This is
the normal npm-ecosystem answer (a caret range, not a pin, so patch/minor
bumps flow in on the next install), and is deliberately left to
ordinary dependency-update hygiene rather than a bespoke sync job.
