# Content Management System front-end — edition spec

Stack-specific specification for the Svelte edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth**; this file adds only what is specific to
this edition. CMS-T25 and CMS-T26 (see Delivery, below) both landed
2026-07-31 — the edition's whole scope shipped before this file grew
past `index.md`, so it stays one file rather than splitting into
topic files that would each hold a paragraph.
<!-- PRO-H8, 2026-08-28: this paragraph previously promised to "grow
topic files … as CMS-T25/T26 land"; spec/tasks.md shows both landed
2026-07-31 and no topic-file split followed. Corrected during the
professionalization sweep rather than left as an open promise. -->

## Stack

SvelteKit 2 · Svelte 5 **runes only** · TypeScript strict · SPA
mode + same-origin BFF proxy · vitest + Playwright. Copy-adapt from
the sibling family front-ends (drift-accepted; the CRM front-end's
BFF proxy + i18n and the PPM front-end's board/grid patterns are
the closest sources). BFF auth per
[../../spec/auth.md](../../spec/auth.md).

## Edition-specific decisions

<!-- PRO-H8, 2026-08-28: heading previously read "(planned)"; every
bullet below is delivered — see Delivery below (CMS-T25/T26, both
landed 2026-07-31 per spec/tasks.md). Corrected during the
professionalization sweep. -->

- **Block editor**, not a rich-text blob: the editor manipulates
  the structured block model directly and posts blocks; no
  `contenteditable`-to-HTML serialization, and no `{@html}` on
  server content ([../../spec/authoring.md](../../spec/authoring.md)).
- **Conflict UI is first-class**: a `409` from a stale
  `base_revision_pid` renders a comparison against the competing
  revision, never a silent retry.
- **Published-vs-draft indicator** on every entry: which revision is
  live, and how far the draft has moved past it.
- **Revision history** with block-level diff and restore (restore is
  a new revision — the UI says so).
- **Preview is server-side**: the BFF holds the preview token and
  proxies the render; the token never reaches a shareable client
  URL ([../../spec/auth.md](../../spec/auth.md)).
- **Asset upload prompts alt text** and explains the publish gate
  when it refuses ([../../spec/assets.md](../../spec/assets.md)).
- **Locale matrix** per entry (status per locale) with staleness
  shown as "N source revisions behind", linking to the diff.
- **Insights render the rule** that produced each finding, plus
  `as_of`; `null` ratios show a no-data state.
- **13-locale i18n from the start** with the parity test; RTL
  (ar / ur) handled by the app's own `lang`/`dir` effect
  (`applyDir` off on Lily's `LocalePicker`, the family workaround).

## Delivery

**CMS-T25 and CMS-T26 landed 2026-07-31** — app shell, BFF proxy +
session flow, typed API client (paths checked against the running
service's OpenAPI document, **verbs included**), 13-locale i18n with
the parity test, Lily locale/theme pickers, and all seven authoring
views. See [../../spec/tasks.md](../../spec/tasks.md).

Two edition facts worth recording here:

- **Lily's helper packages were renamed** `*-select` → `*-picker`
  (`LocalePicker` / `ThemePicker`). Prop contracts are unchanged, but
  the sibling front-ends' `file:` dependency paths are stale — copying
  one today fails to install.
- **The BFF proxy refuses the preview-token surface** rather than
  forwarding it, and `/preview/{pid}/{locale}` does the mint → render
  → revoke round trip server-side.
- **Two pure modules carry the client's whole vocabulary**:
  `$lib/blocks` (the block model — deliberately with no `toHtml` or
  `fromHtml`) and `$lib/format` (the honesty rules: `null` ratios,
  suppressed percentiles, and staleness that keeps *unknown* distinct
  from *up to date*). Both are exhaustively unit-tested, so the views
  stay thin.
