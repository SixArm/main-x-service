# Module 4 — Localization & translation

Locale codes are the family vocabulary
([locales](../../agents/share/locales.md)), optionally with a
region subtag (`fr-CA`). A site declares its `locales[]`, a
`default_locale`, and a **fallback chain** per locale.

## One entry, one variant per locale

An **Entry** is the content identity; an **EntryVariant** is that
content in one locale, and the variant is the unit of workflow: its
own revisions, its own status, its own publish pointer, its own
schedule ([domain-model](domain-model.md)). French can be in review
while English is live — the common case that a single-status model
gets wrong.

Every entry declares a `source_locale` (the language it was
authored in). A variant in another locale carries
`translation_of_revision_pid`: **the exact source revision it was
translated from**. That one field is what makes staleness
computable.

## Fallback chains

A site declares, per locale, an ordered chain ending at the default:
`fr-CA → fr → en`. On delivery the pure core walks the chain until
it finds a **published** variant.

The rule that matters: **delivery always reports the locale actually
served**, in the payload (`locale_served`, `locale_requested`,
`fallback_applied`) and in a `Content-Language` header. A CMS that
silently serves English under a `/fr/` URL is not localized, it is
lying, and readers discover the lie faster than editors do.

A site may also declare `strict_locales[]` — locales for which
fallback is **refused** (`404` instead), for cases where showing
another language is worse than showing nothing (legal notices,
safety information).

## Translation workflow

Per variant, alongside the editorial lifecycle
([workflow](workflow.md)):

```
(none) ──request──▶ requested ──claim──▶ in_translation
                          ──complete──▶ translated ─▶ (ordinary editorial review + publish)
```

- `request` records the source revision, the target locale, the
  requesting actor, and an optional due date; it emits
  `translation_requested`.
- `complete` writes an ordinary revision in the target variant and
  stamps `translation_of_revision_pid` with the source revision it
  was made from; it emits `translation_completed`.
- A translated variant then goes through the **same** review and
  publish path as any other content. Translation status is
  orthogonal to editorial status, not a parallel universe.
- **No machine translation in v1** ([scope](scope.md)); the seam
  where a provider would attach is a documented roadmap item, and a
  machine-produced draft would be marked as such rather than
  presented as a human translation.

## Staleness is derived, never asserted

A translated variant is **stale** when its source variant has
published revisions newer than the one it was translated from:

```
stale  ⇔  source.published_revision.number
             > variant.translation_of_revision.number
```

The derivation reports **how many source revisions behind** and
which ones, so an editor can read the diff rather than re-translate
blindly. Staleness is:

- surfaced per variant on read, in the content-health insights, and
  in the delivery payload's editorial metadata (for authorized
  callers only — a public reader is not told the page is stale);
- **never stored as an editable flag** ([design](design.md)
  CMS-D13);
- **not** an automatic unpublish. Stale-but-published is usually
  better than absent, and that judgement belongs to an editor. A
  site may opt into `unpublish_on_stale` per content type where it
  does not (safety notices again).

## What localization does not do

- No machine translation, no translation memory, no glossary
  enforcement (roadmap).
- No per-locale content *types* — the schema is shared; only values
  vary. A locale needing different fields is a different content
  type.
- No right-to-left rendering concerns in the service: delivery is
  structured JSON; the channel handles direction (the front-end
  edition does, in all 13 locales, from day one).
