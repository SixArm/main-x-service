# Web tier specification

Source of truth for the server-rendered web UI shared across the five
Main X Index service crates (person / worker / place / thing / event).

Each crate ships an identical `src/web/` + `assets/views/` +
`assets/static/css/lily.css` (modulo `AppContext::*()` constructor and
the runtime `entity_singular` / `entity_plural` branch points in
`seed_healthcare`, FHIR resource type, etc). The five matcher crates
(person-matcher, worker-matcher, place-matcher, thing-matcher,
event-matcher) are library-only and do not participate in this web
tier.

For per-crate stack and shared docs, see
[agents/share/web-stack.md](agents/share/web-stack.md). This file is
the authoritative inventory of what exists, where it lives, and what
its contract is. When templates / controllers / CSS diverge from this
file, **this file wins** — update the code to match, or update this
file with the rationale.

## Cross-crate uniformity invariant

The following files **must be byte-identical across all 5 service crates**:

```
assets/views/audit.html.tera
assets/views/audit_recent.html.tera
assets/views/compare.html.tera
assets/views/consents.html.tera
assets/views/edit.html.tera
assets/views/error.html.tera
assets/views/export.html.tera
assets/views/fhir.html.tera
assets/views/health.html.tera
assets/views/home.html.tera
assets/views/import.html.tera
assets/views/index.html.tera
assets/views/layout.html.tera
assets/views/links.html.tera
assets/views/metrics.html.tera
assets/views/not_found.html.tera
assets/views/review_queue.html.tera
assets/views/search.html.tera
assets/views/settings.html.tera
assets/views/show.html.tera
assets/views/tour.html.tera
assets/views/partials/search.html.tera
assets/static/css/lily.css
src/web/controllers.rs
src/web/routes.rs
```

The following file is **per-crate** (it only differs in the
`AppContext::*()` constructor):

```
src/web/views.rs
```

Per-crate runtime branching happens at the Rust level via
`state.app.entity_singular` (`"person"` / `"worker"` / `"place"` /
`"thing"` / `"event"`) and the Tera context variables
`entity_singular` / `entity_plural` / `app_display`.

When syncing, the canonical source is `person-service-rust-crate/`;
copy from there to the other 4 service crates.

Verify uniformity:

```bash
for f in <each shared file>; do
  hashes=$(md5 -q *-service-rust-crate/$f | sort -u | wc -l | tr -d ' ')
  echo "$hashes $f"
done
# Every line must read "1 <path>".
```

## URL surface

| Method + path | Handler | Notes |
|---|---|---|
| `GET /` | `home` | Landing page with mockup showcase |
| `GET /health` | `health` | System RAG + subsystem table + incident timeline |
| `GET /metrics` | `metrics` | Performance dashboard with sparklines (`?range=1h\|6h\|24h\|7d`) |
| `GET /audit` | `audit_recent` | System-wide audit log (`?action=`, `?user_id=`, `?limit=`) |
| `GET /settings` | `settings` | Per-browser preferences (localStorage) |
| `GET /tour` | `tour` | 8-stop guided onboarding tour |
| `GET /docs` | `docs` | API documentation landing with collapsible endpoint groups |
| `GET /palette` | `palette` | Full NHS-token color customizer with live preview |
| `GET /notifications` | `notifications` | Persistent notification center with severity filter + mark-read / dismiss |
| `GET /tokens` | `tokens` | Personal API tokens with `secret-input` reveal/hide/copy + just-generated one-time-reveal banner + generate-token `drawer` + per-row Revoke |
| `GET /dev/500` | `dev_error` | Verifies the 500 page renders |
| `GET /{entity_plural}` | `index` | Data-table + bulk-select + `?page=N` |
| `GET /{entity_plural}/import` | `import` | 5-step CSV import wizard |
| `GET /{entity_plural}/calendar` | `calendar` | Records-by-creation-date month grid (`?month=YYYY-MM`) |
| `GET /{entity_plural}/map` | `map` | Schematic equirectangular world grid with pin markers |
| `GET /{entity_plural}/review-queue` | `review_queue` | Dedup queue (`?status=`, `?quality=`, `?page=N`) |
| `GET /{entity_plural}/review-queue/kanban` | `review_queue_kanban` | Kanban board view with HTML5 drag-and-drop between status columns |
| `GET /{entity_plural}/deduplicate` | `deduplicate` | 4-step batch-dedup wizard: config (range sliders + dry-run) → PIN verify (`pin-input-div`, demo `1357`) → simulated running → results summary |
| `GET /{entity_plural}/trash` | `trash` | Soft-deleted records: paginated data-table with restore + permanent-delete per-row, bulk `dropdown-menu` (Restore / Delete forever / Export JSON) |
| `GET /{entity_plural}/starred` | `starred` | Per-browser starred records; Alpine `x-show`-filters server-seeded candidates against `localStorage["lily-starred-{plural}"]` |
| `GET /{entity_plural}/compare` | `compare` | Side-by-side match (`?a=&b=&review_id=`) |
| `GET /{entity_plural}/search` | `search_page` | Full-page search (`?q=&fuzzy=&phonetic=&mask_sensitive=&page=N`) |
| `GET /{entity_plural}/search/partial` | `search_partial` | HTMX fragment |
| `GET /{entity_plural}/:id` | `show` | Detail page (action-bar: Edit / Audit / Export / FHIR / Consents / Links / Delete) |
| `GET /{entity_plural}/:id/edit` | `edit` | Edit form with 4 list editors (+ healthcare gates) |
| `GET /{entity_plural}/:id/audit` | `audit` | Per-record timeline log |
| `GET /{entity_plural}/:id/export` | `export` | GDPR Article 15 JSON export |
| `GET /{entity_plural}/:id/fhir` | `fhir` | FHIR R5 resource view |
| `GET /{entity_plural}/:id/consents` | `consents` | Consent management (revoke / grant) |
| `GET /{entity_plural}/:id/links` | `links` | Replaces / ReplacedBy / Refer / Seealso graph |
| `GET /{entity_plural}/:id/qr` | `qr` | QR-code share view (server-rendered SVG placeholder) |
| `GET /{entity_plural}/:id/quality` | `quality` | Data-quality score with star/face ratings + improvement suggestions |
| `GET /{entity_plural}/:id/sign` | `sign` | Signature capture (new `signature-pad` canvas + per-stroke undo + typed-name + acknowledgement gate); `?purpose=consent\|witness\|acknowledgement\|authorisation\|other` server-side allowlisted |
| `GET /static/*` | `ServeDir` | Bundled `lily.css`, `htmx.min.js`, `alpine.min.js` |
| _anything else_ | `not_found` | Styled 404 fallback (preserves the requested URI) |

`{entity_plural}` is per crate: `persons`, `workers`, `places`,
`things`, `events`.

### Route ordering

The matchit router resolves literal segments before dynamic ones. In
`src/web/routes.rs`, all of `/review-queue`, `/compare`, `/import`,
`/search`, `/search/partial` are registered **before** `/:id`. The
per-record action paths (`/:id/edit`, `/:id/audit`, `/:id/export`,
`/:id/fhir`, `/:id/consents`, `/:id/links`, `/:id/qr`) sit as siblings of
`/:id` and disambiguate by their suffix.

`Router::fallback(controllers::not_found)` catches anything not in the
table above and returns the styled 404 page with the requested URI
echoed back via the `Uri` extractor.

## Page contracts

Per-page contracts — route, headless components, Tera context shape,
seeded scaffold data, and "real wiring" notes — live in
**[`agents/share/web-pages.md`](agents/share/web-pages.md)** for all
26 pages (home, index, show, edit, audit, export, fhir, consents,
sign, links, review_queue, deduplicate, starred, trash, compare,
search, search/partial, audit_recent, health, metrics, settings,
tokens, tour, import, not_found, error).

When adding or changing a page, edit web-pages.md and the table in
the URL surface section above; templates and controllers MUST stay
consistent with both.

## Layout (`layout.html.tera`)

The layout is the centerpiece. It carries:

### Markup

- `<html lang="en">` — the early-bootstrap script may override `lang`
  + `dir` from `localStorage["lily-locale"]`
- `<head>` — title block, CSS link, HTMX + Alpine scripts (both
  deferred), early-bootstrap inline script
- `skip-link` to `#main-content`
- `header.header[aria-label="Site header"]` containing:
  - `<h1>{{ app_display }}</h1>`
  - `nav.navigation-menu` with the primary nav `<ul>` (Home / Plural /
    Search / Review queue / Audit / Health / Metrics / Tour /
    Settings)
  - `.theme-picker` with `theme-select` (NHS UK / Dark / High
    contrast)
  - `.locale-picker` with a 47-option `select` (English + 46 locales
    from `agents/share/locales.md`)
- `main#main-content.page-wrapper` carrying `{% block content %}`
- `footer.footer[aria-label="Site footer"]` with the
  Lily/Loco/Tera/HTMX/Alpine attribution
- `dialog#shortcuts-dialog.dialog[role="dialog"]` — the keyboard-
  shortcut overlay, opened by `?`
- `div#htmx-indicator` — fixed-position `progress-spinner` + "Loading…"
  label, toggled by the HTMX-indicator bridge
- `div#toast-region.sonner[role="status"][aria-live="polite"]` — the
  toast container

### Inline JS bridges

All bridges live in **one inline `<script>` block** at the bottom of
the layout. They are independent IIFEs and use only standard browser
APIs. They never throw on failure: `localStorage` access is wrapped in
`try`/`catch`.

| Bridge | Responsibility |
|---|---|
| Toast | Exposes `lily.toast(message, type)`; appends a `.toast[data-type="…"]` to `#toast-region`; auto-removes after 5 s. Also listens for the HTMX `showToast` event so controllers can emit `HX-Trigger: {"showToast":{"message":"…","type":"success"}}`. |
| HTMX-indicator | Tracks an in-flight request counter via `htmx:beforeRequest` / `htmx:afterRequest` / `htmx:responseError` / `htmx:sendError`; toggles `#htmx-indicator[data-active="true"]`; spawns error toasts on `responseError` / `sendError`. |
| Hover-card | Delegated `mouseover` / `mouseout` / `focusin` / `focusout` on `document`; toggles `data-open="true"` on `.hover-card` matching `aria-describedby` of a sibling `.hover-card-trigger`. Escape closes any open card. |
| Keyboard shortcuts | Listens for `keydown` document-wide. `Ctrl+K`/`Cmd+K` toggles `#command-palette` (works inside inputs); `?` toggles `#shortcuts-dialog`; `/` focuses the first `[type="search"]` / `[type="text"]`; `g` followed by one of `h/i/s/r/a/e` within 1500 ms navigates to home / index / search / review-queue / audit / health. All non-`Ctrl/Cmd+K` bindings suppressed inside `<input>` / `<textarea>` / `<select>` / contenteditable. |
| Command palette | `<dialog id="command-palette">` with an Alpine-driven `command` list of 20 seeded actions; filter as you type; ↑/↓ navigate; Enter invokes; click also works. Actions dispatch to `href` URLs, theme switching, palette reset, or open the shortcuts dialog. |
| Theme picker | Syncs `#theme-select` to the current `[data-theme]`; persists changes to `localStorage["lily-theme"]`; spawns confirmation toast. |
| Locale picker | Syncs `#locale-select` to the current `[lang]` (treating default `en` as the empty option); persists to `localStorage["lily-locale"]`; sets `lang` + `dir` (RTL for ar / fa / ur); spawns confirmation toast. |
| Clipboard | Delegated `click` on `.clipboard-copy-button`; reads `data-clipboard-text` (or textContent); writes via `navigator.clipboard.writeText`; sets `data-copied="true"` for ~2 s; spawns success / error toast. |

### Head FOUC bootstrap

A short script in `<head>` (before `<body>`) reads two
`localStorage` keys synchronously:

- `lily-theme` — if `dark` or `high-contrast`, sets
  `<html data-theme="…">`
- `lily-locale` — if matches `^[a-z]{2}$`, sets `<html lang="…"
  dir="ltr|rtl">` (RTL for ar / fa / ur)

Both are wrapped in `try` / `catch`; missing or invalid values are
silently ignored.

## Headless component inventory

The canonical, exhaustive component inventory — tag, class, required
ARIA, which page uses each — lives in
**[`agents/share/web-stack.md`](agents/share/web-stack.md)**. Treat
that table as the source of truth. When a new headless component is
added or removed, update the web-stack table and reference it from the
relevant page contract above.

Component families used by the web tier, at a glance:

- **Navigation / chrome**: `header`, `footer`, `skip-link`,
  `navigation-menu`, `breadcrumb-nav`, `pagination-nav`, `tree-nav`,
  `accordion-nav`, `tab-bar`, `tour-list`, `command`, `chat-nav`
- **Layout / containers**: `card`, `section`, `hero`, `diff`,
  `mockup-browser`, `mockup-shell`, `mockup-phone-portrait`,
  `aspect-ratio-container`, `kanban-table`, `calendar-table`,
  `summary-list`, `data-table`, `timeline-list`, `details`
- **Forms / inputs**: `form`, `field`, `label`, `hint`, `fieldset`,
  `text-input`, `search-input`, `tel-input`, `email-input`,
  `checkbox-input`, `select`, `option`, `pin-input-div`,
  `range-input`, `color-input`, `file-input`, `file-upload`,
  `tag-input`, `address-input`,
  `united-kingdom-national-health-service-number-input`,
  `united-states-social-security-number-input`
- **Actions / buttons**: `button`, `action-bar`, `action-bar-button`,
  `action-link`, `clipboard-copy-button`, `switch-button`,
  `star-button`, `dropdown-menu`, `theme-select`, `color-picker`,
  `segment-group`
- **Status / feedback**: `alert`, `alert-dialog`, `dialog`, `drawer`,
  `super-banner`, `notification`, `sonner`, `toast`,
  `error-summary`, `error-message`, `badge`, `tag-group`,
  `information-callout`, `red-amber-green-view`, `progress`,
  `progress-spinner`, `meter`, `sparkline`, `hover-card`,
  `qr-code`, `five-star-rating-view`, `five-face-rating-view`,
  `code-block`, `kbd`, `secret-input`, `signature-pad`
- **Healthcare overlay** (gated by `{% if healthcare %}`):
  `medical-banner`, `medical-banner-box`,
  `medical-banner-box-for-danger`, `medical-banner-box-for-advice`,
  `care-card`,
  `united-kingdom-national-health-service-number-view`,
  `united-states-social-security-number-view`

## Healthcare overlay

The `show` and `edit` templates each gate a healthcare overlay behind
`{% if healthcare %}`. The Rust `seed_healthcare(state)` helper returns
`Some(...)` only when `state.app.entity_singular == "person"` (the
healthcare-aware service crate); otherwise `None` (and
`emergency_contacts` defaults to an empty `Vec`).

When present, the overlay renders:

- **On `show`**: `medical-banner` summary strip (name + NHS / SSN /
  DOB) + one `medical-banner-box-for-danger` per alert + one
  `medical-banner-box-for-advice` per advice item + one `care-card`
  per care instruction (each tagged with an urgency `badge`).
- **On `edit`**: a `fieldset` with `united-kingdom-national-health-
  service-number-input` + `united-states-social-security-number-input`,
  and a separate **Emergency contacts** `fieldset` whose chip pattern
  is the same `tag-input` shape but with `summary-list-item`-shaped
  chips.

## CSS conventions

The bundled `lily.css` (NHS UK theme) is treated as the consumer-
supplied skin for the Lily HTML Headless contracts. Custom CSS added
during this work, beyond the base NHS theme:

- `:root[data-theme="dark"]` / `:root[data-theme="high-contrast"]` —
  theme skin overrides (token redefinitions plus a few rule patches)
- `.theme-picker`, `.locale-picker` — header pickers (flex, gap)
- `.red-amber-green-view::before` — colored dot keyed by `data-status`
  (`red` / `amber` / `green`)
- `.medical-banner`, `.medical-banner-box`, `.medical-banner-box-for-
  danger`, `.medical-banner-box-for-advice` — NHS-themed medical banner
- `.diff` — 2-column responsive grid (1-col < 768 px)
- `.code-block` — monospace, pale-grey, scrollable
- `.clipboard-copy-button[data-copied="true"]` — green confirmation
- `.hover-card-trigger`, `.hover-card`, `.hover-card[data-open="true"]`
- `.field { position: relative }` — anchor for absolutely-positioned
  hover-cards inside form fields
- `.kbd` — monospace box with bottom-edge shadow
- `.mockup-browser`, `.mockup-shell`, `.mockup-phone-portrait` —
  decorative chrome
- `#htmx-indicator` — fixed top-right spinner pill
- `.drawer[data-open="true"]` — slide-in transform
- `.drawer-backdrop` — click-catcher overlay

## Data attributes used

| Attribute | Used by | Meaning |
|---|---|---|
| `data-theme="dark"`/`"high-contrast"` | `<html>` | Active theme |
| `data-status="red"`/`"amber"`/`"green"` | `.red-amber-green-view` | RAG indicator |
| `data-type="success"`/`"info"`/`"warning"`/`"error"` | `.badge`, `.alert`, `.medical-banner-box-for-*` | Semantic variant |
| `data-context="medical"` | `.medical-banner*` | Marks the medical-banner family |
| `data-required` | `.field` | Renders required asterisk via `::after` |
| `data-open="true"` | `.hover-card`, `.drawer`, `#htmx-indicator` (`data-active`) | Visibility toggle |
| `data-copied="true"` | `.clipboard-copy-button` | Post-copy confirmation |
| `data-clipboard-text` | `.clipboard-copy-button` | Override of textContent |
| `data-copied-message` | `.clipboard-copy-button` | Toast message text |
| `data-selected-count` | `.action-bar` | Bulk-selection count (set by Alpine) |
| `data-selected` | `.data-table-row` | Per-row selection state (set by Alpine) |
| `aria-current="page"` | leaf `breadcrumb-list-item` / current `pagination` span | Per WAI-ARIA |

## Localstorage keys

| Key | Format | Set by | Read by |
|---|---|---|---|
| `lily-theme` | `"dark"` \| `"high-contrast"` (absent = NHS UK) | theme picker | head FOUC bootstrap, theme bridge |
| `lily-locale` | `[a-z]{2}` ISO 639-1 (absent = `en`) | locale picker | head FOUC bootstrap, locale bridge |
| `lily-settings` | JSON | `/settings` Alpine | settings page |
| `lily-tour-steps` | JSON array of step keys | `/tour` Alpine | tour page |
| `lily-accent` | `#RRGGBB` hex (absent = NHS blue) | settings color picker | head FOUC bootstrap, settings Alpine |
| `lily-palette` | JSON map `{ "nhs-blue": "#…", … }` | `/palette` Alpine | `/palette` Alpine (re-applies on `x-init`); CSS custom-property overrides on `<html>` |
| `lily-notifications` | JSON array of notification objects | `/notifications` Alpine | `/notifications` Alpine (read on `x-init`, written on every mutation) |
| `lily-starred-{plural}` | JSON array of record IDs (per entity, e.g. `lily-starred-persons`) | `star-button` `toggleStar()` on `/{plural}` index + `/{plural}/{id}` show + `/{plural}/starred` | all three pages on x-data init |

## Conventions

### Per-crate uniformity

When adding a new view: build it in
`person-service-rust-crate/`, smoke-test, then sync to the other 4
service crates with the standard pattern:

```bash
SRC=/Users/jph/git/sixarm/main-x-service/person-service-rust-crate
for d in event-service-rust-crate \
         place-service-rust-crate thing-service-rust-crate \
         worker-service-rust-crate; do
  DST=/Users/jph/git/sixarm/main-x-service/$d
  cp "$SRC/<file>" "$DST/<file>"
done
```

Then `cargo check --bin web` in each crate (run in parallel via
background tasks).

### Per-crate runtime branching

Branching on entity type happens at the Rust level via
`state.app.entity_singular`. Examples:

- `seed_healthcare(state)` — `Some(...)` only for `"person"` (today;
  reserved for future healthcare-flavoured service crates)
- `fhir(state, id)` — maps `entity_singular` to FHIR resource type
- `tour(state)` — builds per-crate `href` values

Templates never `{% if entity_singular == "person" %}`; they branch
only on the presence of a context variable (e.g. `{% if healthcare
%}`).

### Aria-labels on every interactive headless component

Per the Lily HTML Headless contracts, every interactive element
carries an `aria-label`. Where a visible `<label>` already binds via
`for=`, the input gets its own `aria-label` matching the label text
(not strictly required, but the consistent pattern keeps screen
readers verbose-but-accurate).

### Inline-script entity caveat

HTML5 parses `<script>` content as **raw text** — HTML entities are not decoded. So `&amp;&amp;` written inside `<script>` is a literal 10-character string, not the `&&` operator (and is a JS SyntaxError if it parses at all).

The convention in `layout.html.tera`:
- **Inside `<script>` blocks**: write raw `&&`.
- **Inside Alpine `x-data="…"` HTML attribute values**: write entity-encoded `&amp;&amp;` (Alpine decodes attribute text before evaluation).

A previous bug had `&amp;&amp;` in `<script>` blocks; the surrounding `try`/`catch` silently swallowed the SyntaxError so the breakage was invisible until the command-palette slice exercised `&&` in a non-try-wrapped path.

### HTMX trigger header convention

Controllers can emit `HX-Trigger: {"showToast":{"message":"…","type":"
…"}}` to spawn a toast from a server response. The HX event name is
`showToast`; both `message` and `type` are optional.

### Route precedence

When adding a new route under `/{entity_plural}/...`:

- Literal segments (e.g. `/dashboard`) must be registered **before**
  `/:id` in `routes.rs`.
- Per-record action paths (`/:id/edit`, `/:id/audit`, …) sit as
  siblings of `/:id` and disambiguate by suffix.

## Real-data wiring status

Every page on the web tier is currently a **scaffold**: controllers
seed plausible JSON via `serde_json::json!(...)`. The minimum viable
path to real data is to plumb `AppState` (with the existing
per-crate repository — `PersonRepository`, `WorkerRepository`,
`PlaceRepository`, `ThingRepository`, `EventRepository` — plus
`SearchEngine` / `AuditLogRepository` / event publisher) into
`WebState` and then replace the `seed_*` calls in each handler.

The order of work for real wiring (lowest-risk first):

1. `index` / `show` — read-only list + detail (no mutation, easy 404
   via `not_found`)
2. `audit` / `audit_recent` — read-only timeline
3. `search` / `search_partial` — read-only Tantivy query
4. `health` / `metrics` — pings against the live subsystems + OTLP
   metrics
5. `export` / `fhir` — delegate to `crate::privacy` and
   `crate::api::fhir`
6. `compare` — delegate to `MatcherProbabilistic`
7. `consents` / `links` — read + simple POST/DELETE round-trips
8. `edit` (mutation) — POST/PUT with 422 → re-render with
   `errors`/`field_errors`; HTMX returns the error-summary fragment
9. `review_queue` — wire merge / reject HTMX calls to the live REST
   endpoints
10. `import` — replace the in-browser CSV parse with a streamed
    multipart upload + SSE / HTMX progress channel

## Source files

- Controllers + helpers: `src/web/controllers.rs`
- Routes: `src/web/routes.rs`
- Tera engine + `WebState`: `src/web/views.rs`
- Loco-hooks placeholder: `src/web/app.rs`
- Bin entry: `src/bin/web.rs`
- Templates: `assets/views/*.html.tera`
- CSS: `assets/static/css/lily.css`
- JS: `assets/static/js/{htmx,alpine}.min.js`

## Related docs

- [agents/share/web-stack.md](agents/share/web-stack.md) — per-crate
  stack inventory; component-table summary that mirrors this file's
  inventory section
- [agents/share/locales.md](agents/share/locales.md) — full 46-locale
  table driving the locale picker
- [agents/share/architecture.md](agents/share/architecture.md) —
  system architecture (API + business + DB tiers; the web tier sits
  on top of the existing Axum REST API)
- [agents/share/restful.md](agents/share/restful.md) — REST API surface
  the web tier wraps
- Each crate's `AGENTS/restful.md` — per-crate REST endpoints
