# Svelte front-end stack — Lily package distribution

> **Scope note.** This doc covers the Lily **npm packages** (the picker
> components). The Lily **theme stylesheets** each front-end's
> `static/assets/themes` serves are a separate, narrower problem with
> its own fix — see §8.

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

## 3. Decision: registry versions, under the `@lilydesignsystem` npm org

**Publish, and depend on the published semver version.** This was
first checked 2026-09-16 against the (then-unscoped) package names,
found already true, and adopted family-wide in WEB-2. It moved again
almost immediately: the Lily project itself migrated every package to
a proper npm **organization scope**, `@lilydesignsystem`, dropping the
redundant `lily-design-system-` prefix now that the scope carries that
meaning, and resetting every package to `0.1.0` under the new
namespace (npm scopes are separate registry namespaces — no version
history carries over from the unscoped names, which remain published
but are no longer the ones this family depends on):

| Package (unscoped, retired) | Package (current) | Version |
|---|---|---|
| `lily-design-system-svelte-headless` | `@lilydesignsystem/svelte-headless` | 0.1.0 |
| `lily-design-system-svelte-theme-picker` | `@lilydesignsystem/svelte-theme-picker` | 0.1.0 |
| `lily-design-system-svelte-locale-picker` | `@lilydesignsystem/svelte-locale-picker` | 0.1.0 |
| `lily-design-system-svelte-text-size-picker` | `@lilydesignsystem/svelte-text-size-picker` | 0.1.0 |
| `lily-design-system-svelte-share-picker` | `@lilydesignsystem/svelte-share-picker` | 0.1.0 |
| `lily-design-system-svelte-picker-bar` | `@lilydesignsystem/svelte-picker-bar` | 0.1.0 |

Creating the npm organization itself is a one-time, web-only step (the
npm CLI has no `npm org create` — only `npm org set/rm/ls` for
managing membership in an org that already exists); once it exists,
publishing and depending on it is ordinary npm.

Every front-end's `package.json` (and every `import`/`resolve.alias`
naming these packages) uses the scoped names:

```jsonc
"@lilydesignsystem/svelte-headless": "^0.1.0",
"@lilydesignsystem/svelte-locale-picker": "^0.1.0",
"@lilydesignsystem/svelte-picker-bar": "^0.1.0",
"@lilydesignsystem/svelte-share-picker": "^0.1.0",
"@lilydesignsystem/svelte-text-size-picker": "^0.1.0",
"@lilydesignsystem/svelte-theme-picker": "^0.1.0",
```

`pnpm install --frozen-lockfile` then succeeds on a clean runner with
no sibling checkout — the property WEB-2's CI stage needs — exactly as
it did under the unscoped names; the scope migration changes the
package identity, not this property.

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

## 8. The theme stylesheets: the `@lilydesignsystem/themes` package

The 45 Lily theme stylesheets are published as
[`@lilydesignsystem/themes`](https://www.npmjs.com/package/@lilydesignsystem/themes)
(`dist/<theme>.css`, exported as `./*.css`). Every front-end, and the
GitHub Pages site, depends on it at an **exact** version (`0.1.0`) and
serves it from `static/assets/themes`, which is a symlink to
`../../node_modules/@lilydesignsystem/themes/dist`. The URL the ThemePicker
loads (`/assets/themes/<slug>.css`) is unchanged, and `pnpm run build` copies
the files through Vite's static-asset handling exactly as before.

History, because it explains the shape. The themes were first symlinks onto
a sibling checkout, which broke `svelte-kit sync` on any CI runner (`ENOENT`
on a dangling symlink, found rolling out the `front-end` CI stage, WEB-2);
that was fixed by a **vendored** copy (`vendor/lily-design-system-themes/`)
because no package existed yet. The package now does, so the vendored copy
is deleted and there is nothing to re-sync by hand.

Consequences worth knowing:

- **The symlink dangles until `pnpm install` has run.** CI installs before
  it checks or builds, so nothing changes there; a fresh clone needs
  `pnpm install` before `svelte-kit sync` (which the `dev`, `check` and
  `build` scripts already presuppose for the Lily picker packages).
- **A theme change is a version bump**, made in one place per project:
  `pnpm add --save-exact @lilydesignsystem/themes@<version>`. The pin is
  exact on purpose, so a theme edit never arrives as a surprise patch bump;
  Dependabot proposes the bump like any other dependency.
- **The GitHub Pages site** exports via `git subtree split`, so its
  `static/assets/themes` must resolve inside the subtree: it does, because
  the target is its own `node_modules`, not a path outside the directory.
- To try an in-flight theme change from a Lily checkout, use `pnpm link`
  for that package, as for the picker packages (§ above); it touches no
  committed file.
