# Vendored Lily Design System themes

A vendored copy of `themes/*.css` from the Lily Design System repo
(`~/git/lilydesignsystem/lily-design-system/themes/` in this machine's
layout — a sibling checkout, not part of this repository).

## Why this exists

Every one of the sixteen SvelteKit front-ends' `static/assets/themes`
was a symlink onto that sibling checkout. That works for local
development (themes update live) but is fatal for CI: a clean runner
has no such checkout, so `svelte-kit sync` fails outright trying to
`stat` a dangling symlink (`ENOENT`) — found rolling out the
`front-end` CI stage (`tasks.md` WEB-2), the same class of problem
`agents/share/svelte-front-end-stack.md` already solved for the Lily
**packages** (switched to published npm versions) but that fix did not
reach these theme **stylesheets**, which are not published anywhere —
there is no `lily-design-system-themes` npm package.

Each front-end's `static/assets/themes` is now a symlink onto **this**
in-repo copy instead of the external one, so `pnpm run build` still
copies the theme stylesheets through Vite's static-asset handling
exactly as before, but with nothing outside this repository in the
resolution path.

## Keeping this in sync

This is a **vendored snapshot**, not a live link — a theme added or
edited in the Lily repo does not appear here until someone re-copies
it:

```sh
cp ~/git/lilydesignsystem/lily-design-system/themes/*.css \
   vendor/lily-design-system-themes/
```

Vendored once (all sixteen front-ends share this single copy — not
sixteen copies) rather than published, for the same reason
`entity-ref` and `integrity-mac` are real dependencies rather than
copies: one vendored directory that only a human resync touches is a
much smaller drift surface than one per project. If this needs to
change more often than "rarely," publishing a real
`lily-design-system-themes` npm package (mirroring the packages doc's
own resolution for the picker components) is the better fix — record
that decision in `agents/share/svelte-front-end-stack.md` if it
happens, not here.
