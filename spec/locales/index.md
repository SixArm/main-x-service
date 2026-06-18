# Localization & Internationalization (i18n / l10n) — monorepo-wide spec

> **Scope.** This is the family-wide specification for **localization
> (l10n)** and **internationalization (i18n)** across the Main X Index.
> It describes the supported-languages reference, the *one* place i18n
> is actually implemented today (the authentication service email +
> the authentication front-end UI), the lightweight catalog **pattern**
> the rest of the family is expected to follow, and the honest coverage
> gap (every other front-end and service is English-only). It sits
> *above* the per-crate specs; where a detail is load-bearing it is
> restated here, but the per-crate spec remains the source of truth for
> its crate.

Related sibling topics:
[authentication](../authentication/index.md) ·
[validation](../validation/index.md) ·
[REST conventions](../restful/index.md).

Family-wide supported-language reference:
[agents/share/locales.md](../../agents/share/locales.md).

---

## 1. Scope & vocabulary

This document distinguishes two words that are often conflated:

| Term | Meaning here |
| ---- | ------------ |
| **Internationalization (i18n)** | The *engineering* discipline: separating user-facing strings from code, keying them, selecting a locale at runtime, formatting dates/numbers/text by locale, and supporting bidirectional layout. A codebase is "internationalized" once it can hold more than one language without code changes. |
| **Localization (l10n)** | The *content* act: supplying the translated strings (and locale-specific formats) for a given language. A product is "localized to Welsh" when the `cy` catalog is complete. |
| **Locale** | A language (and optionally region/script) identity used to pick a catalog. The family keys catalogs on the **primary language subtag** (`cy`, not `cy-GB`); see §5.1. |
| **Catalog** | A per-locale map of stable keys → translated strings. The family uses plain in-language maps (no `.po`/`.json` resource-bundle machinery). |
| **Fallback** | The locale used when the requested one is missing or a key is untranslated. Family-wide this is **English (`en`)**. |

### 1.1 Supported-language reference

The canonical list of languages the family recognizes is the **ISO
639-1** table in
[agents/share/locales.md](../../agents/share/locales.md) — 46 languages
spanning European, South-Asian, East-Asian, Middle-Eastern, and African
tongues (e.g. `ar` Arabic, `cy` Welsh, `de` German, `hi` Hindi, `ja`
Japanese, `zh` Mandarin, …), each with its endonym and English name.

That table is a **recognition reference**, not a coverage claim: it
lists the codes the family considers valid and the languages it intends
to be able to serve. A language appearing there does **not** mean any
catalog has been translated into it. Actual shipped coverage is §2.

---

## 2. Implemented today

Exactly one product area in the family is internationalized **and**
localized today: **authentication**. Both halves ship **English (`en`)**
plus **Welsh (`cy`)** — Welsh chosen for the UK public-sector
Welsh-language duty (Welsh Language (Wales) Measure 2011 — treat Welsh
"no less favourably than English").

### 2.1 Authentication **service** — localized magic-link email

Reference implementation:
[`authentication-service-with-loco/src/i18n.rs`](../../authentication/authentication-service-with-loco/src/i18n.rs)
(see also the service
[spec](../../authentication/authentication-service-with-loco/spec/index.md)).

The magic-link email is the only user-facing text the service emits, and
it is localized. The module is a **pure, dependency-free** catalog: a
function over a locale string, testable without a database or running
app.

| Element | Value |
| ------- | ----- |
| Catalog entry | `magic_link_email(locale) -> EmailStrings { subject, text, html }` |
| Translated locales | `SUPPORTED_LOCALES = ["en", "cy"]` |
| Default / fallback | `DEFAULT_LOCALE = "en"` |
| Placeholder | bodies carry a single `{link}` token; `EmailStrings::render(link)` substitutes it. The URL is locale-independent. |

**Locale selection.** `select_locale(preference: Option<&str>) ->
String` resolves the locale to send the email in:

1. The caller's preference is the request body's optional **`locale`**
   field (see the service spec §6). `Accept-Language` is the natural
   secondary source and uses the same normalization.
2. `normalise_locale` lower-cases, trims, splits on `,`/`;` to take the
   first preference, and drops any region/script subtag
   (`cy-GB` → `cy`, `EN_us` → `en`).
3. Unknown, malformed, or absent input falls back to `en`.

This is enforced by tests: unknown locales (`zz`, `fr`, `de`, ``,
`not-a-locale`) all render the English copy; region subtags reduce to
the primary language; and a guard test asserts every non-default
supported locale ships **distinct** copy (so a half-added locale that
silently renders English is a test failure).

### 2.2 Authentication **front-end** — localized SPA UI

Reference implementation:
[`authentication-front-end-with-svelte/src/lib/i18n.svelte.ts`](../../authentication/authentication-front-end-with-svelte/src/lib/i18n.svelte.ts)
(see also the front-end
[spec](../../authentication/authentication-front-end-with-svelte/spec/index.md)).

A **dependency-free** i18n module (no i18n library — the SPA keeps its
deps minimal). Structure mirrors the service catalog:

| Element | Value |
| ------- | ----- |
| Catalog | `STRINGS: Record<Locale, Record<StringKey, string>>` keyed by stable dotted keys (`signin.title`, `account.signout`, `verify.error.invalid`, …) |
| Translated locales | `LOCALES = ["en", "cy"]` |
| Switcher labels | `LOCALE_LABELS = { en: "English", cy: "Cymraeg" }` (each in its own language) |
| Default / fallback | `DEFAULT_LOCALE = "en"` |
| Reactive locale | `let current = $state<Locale>(...)` (Svelte 5 runes) |
| Accessor | `t(key)` — reactive; reads `current`, so switching locale re-renders every string |
| Pure variant | `translate(key, locale)` — unit-testable without a component |

**Locale switcher + persistence.** `i18n.set(next)` normalizes input
(same `cy-GB` → `cy`, unknown → default rule as the service), updates
the reactive `current`, and persists the choice to **localStorage** under
`mxi.auth.locale`. On load, `readStoredLocale()` re-seeds `current` from
localStorage (defaulting off the browser, e.g. during SSR/SPA boot).

**End-to-end coherence.** The chosen UI locale is sent as the `locale`
field on signup / magic-link requests, so the **email language matches
the UI language** — the service-side selection (§2.1) and the
front-end selection are the same value travelling across the API.

**Key-set integrity.** `StringKey` is derived from the English catalog
(`keyof STRINGS["en"]`), so every other locale must cover the same keys
or it is a TypeScript error. `translate` falls back locale → English →
the key string itself, so a missing translation degrades gracefully
rather than throwing.

### 2.3 Coverage snapshot

| Area | i18n'd? | Locales shipped | Mechanism |
| ---- | ------- | --------------- | --------- |
| authentication service (magic-link email) | ✓ | `en`, `cy` | `src/i18n.rs` catalog + `select_locale` |
| authentication front-end (SPA UI) | ✓ | `en`, `cy` | `src/lib/i18n.svelte.ts` catalog + reactive `t()` + switcher |
| all other service crates | ✗ | `en` only | hard-coded English in handlers/messages |
| all other (operator) front-ends | ✗ | `en` only | hard-coded English in components |

---

## 3. The pattern to follow

Both authentication halves embody one deliberate pattern. New i18n work
in the family should copy it rather than reaching for a framework.

1. **Dependency-light catalog.** A per-locale map of stable keys →
   strings, defined in code. **No heavy i18n library**, no ICU
   MessageFormat runtime, no `.po`/gettext toolchain. The surface is
   small; keep the dependency footprint small (drift is accepted —
   copy-adapt from authentication rather than factoring out a shared
   package).
2. **English is the source of truth.** `en` defines the full key set;
   every other locale is measured against it. Missing keys fall back to
   `en`, then (front-end) to the key itself. A locale that silently
   renders English is treated as a bug (the service has a guard test for
   exactly this).
3. **Locale normalization is shared-shaped.** Trim, lower-case, take the
   first listed preference, drop the region/script subtag, match on the
   primary language subtag; unknown → default. The Rust and TypeScript
   implementations are intentionally the same algorithm.
4. **Locale is selected and persisted.** Front-end: reactive `$state` +
   localStorage. Service: explicit `locale` request field (and/or
   `Accept-Language`). The two agree so user-facing output is coherent
   across UI and email.
5. **Adding a locale is a one-place edit.** Extend the `SUPPORTED_LOCALES`
   / `LOCALES` list and add the matching catalog entry (plus a switcher
   label on the front-end). No control-flow changes; the fallbacks and
   the key-set check do the rest.

---

## 4. Current coverage gap (honest)

i18n in this family is a **per-project rollout, not a family-wide
capability**. Today:

- Only the **authentication** service and front-end are
  internationalized and localized (`en` + `cy`).
- Every **operator front-end** (person, worker, place, thing, event,
  course, organization, care-pathway, case, case-folder) is
  **English-only** — strings are hard-coded English in the components,
  with no catalog, no locale switcher, and no localStorage locale.
- Every other **service crate** emits **English-only** user-facing text
  (validation messages, error strings, audit descriptions). None has an
  `i18n` module.

Nothing structurally blocks extending the pattern — it simply has not
been done. There is no shared i18n package by design (consistent with
the family's accepted front-end drift); each project that needs
localization will copy-adapt the authentication catalog.

---

## 5. Considerations

### 5.1 BCP-47 vs ISO 639-1

Two different notions of "language tag" coexist in the family, and they
must not be confused:

| Use | Standard | Where |
| --- | -------- | ----- |
| **UI / email locale selection** | **ISO 639-1** primary subtag | authentication catalogs key on `en` / `cy`; region/script subtags are *dropped* during normalization |
| **Domain data field** (`in_language` on an entity) | **BCP-47** tag *syntax* | care-pathway / case validation (see [validation](../validation/index.md) §"BCP-47 language tag syntax") |

The distinction:

- **Locale selection** answers "which language do we *speak to this
  user* in?" — a small, closed set we have actually translated. It
  collapses `cy-GB` → `cy` because the catalog is per-language, not
  per-region.
- **`in_language` validation** answers "is this *data field describing
  the record's content language* a well-formed tag?" — an open set
  validated for **BCP-47 syntax** (language subtag, optional
  script/region/variant), *not* limited to the languages we serve. The
  [validation spec](../validation/index.md) covers the exact rule and
  notes IANA subtag-registry **existence** checking as a deferred,
  stricter step.

A language can be valid `in_language` data (e.g. a Welsh-language care
pathway tagged `cy`) while the UI serving it remains English — the two
axes are independent.

### 5.2 Right-to-left (RTL) languages

Several languages in the [reference table](../../agents/share/locales.md)
are right-to-left: `ar` (Arabic), `fa` (Persian), `ur` (Urdu) — and the
common addition `he` (Hebrew, not yet in the table). RTL is **not** just
translated strings; it requires layout direction (`dir="rtl"`),
mirrored components, and logical (start/end) CSS rather than
left/right. No front-end currently sets `dir` or handles RTL. Adopting
any RTL locale is a layout project, not only a catalog addition, and
should be specced per front-end before the locale is offered.

### 5.3 Locale-aware formatting

Catalogs cover *strings*; they do not cover **formatting**. Dates,
times, numbers, and currency render the same regardless of locale today.
A fully localized experience needs locale-aware formatting:

- Front-end: `Intl.DateTimeFormat` / `Intl.NumberFormat` keyed off the
  active locale (already available in the browser, no dependency).
- Service: any human-readable timestamps/numbers it emits would need
  locale-aware rendering; currently it emits machine formats (ISO-8601,
  JSON numbers) that are correctly locale-neutral on the wire.

Wire formats stay locale-neutral (ISO-8601 timestamps, decimal
numbers); formatting is a **presentation-layer** concern.

### 5.4 Translation workflow

With in-code catalogs and no `.po`/TMS pipeline, the workflow is
deliberately simple and PR-based:

1. Add the locale code to the supported list and a catalog entry
   covering the full English key set.
2. Add a switcher label (front-end) written in the new language.
3. The key-set check (TS `StringKey`; Rust distinct-copy guard test)
   catches incomplete or stub translations.
4. Review the translation in the same three-part PR as the spec/code
   edit, per the family's SDD discipline.

If the translated surface grows large enough that hand-maintained maps
become unwieldy, revisit a resource-bundle/TMS approach as an explicit
spec change — but only then.

---

## 6. Roadmap

i18n is intentionally implemented-narrow today; the roadmap is to widen
it along the established pattern, **not** to introduce new machinery.

1. **Extend the catalog pattern to the operator front-ends.** Lift the
   `i18n.svelte.ts` shape (reactive `$state` locale, `t()` accessor,
   localStorage persistence, switcher) into a front-end that needs a
   second locale, copy-adapting rather than sharing a package. Welsh is
   the natural first target given the public-sector duty.
2. **Localize service-generated user-facing text.** Give the other
   service crates an `i18n.rs`-style catalog for the human-readable text
   they emit (validation messages, notification/email copy), selected
   via a `locale` field and/or `Accept-Language` exactly as
   authentication does.
3. **Add locales by extending the supported list.** Each new language is
   a catalog entry plus (front-end) a switcher label; the existing
   fallback + key-set guards keep partial translations safe.
4. **Treat RTL and locale-aware formatting as their own milestones**
   (§5.2, §5.3) — adopt them per front-end, ahead of offering any RTL
   locale, rather than bundling them into a string-only catalog add.

Until those land, English remains the family-wide default and the only
guaranteed locale outside authentication.
