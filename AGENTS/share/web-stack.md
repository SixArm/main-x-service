# Web stack: Loco, Tera, HTMX, Alpine, Lily (HTML Headless)

Each Main X Index crate ships a server-rendered web UI on top of the existing Axum REST API. The stack favors progressive enhancement: the server returns HTML, HTMX swaps fragments on user interaction, Alpine adds small client-side state, and the Lily Design System (HTML Headless) provides accessible component structure and ARIA semantics. Visual styling is supplied by the consumer (currently the NHS UK theme bundled as `lily.css`).

## Components

| Layer | Tool | Version | Where |
|-------|------|---------|-------|
| Web framework conventions | [Loco.rs](https://loco.rs) | 0.14 | `config/*.yaml`, `src/web/app.rs` |
| HTTP server | [Axum](https://github.com/tokio-rs/axum) | 0.7 | `src/web/routes.rs`, `src/bin/web.rs` |
| Templates | [Tera](https://keats.github.io/tera/) | 1.20 | `assets/views/**/*.tera` |
| Static-file middleware | [tower-http](https://github.com/tower-rs/tower-http) `ServeDir` | 0.6 (`fs` feature) | mounted at `/static` |
| Server-driven AJAX | [HTMX](https://htmx.org) | 2.0.4 | `assets/static/js/htmx.min.js` |
| Client-side reactivity | [Alpine.js](https://alpinejs.dev) | 3.14.8 | `assets/static/js/alpine.min.js` |
| Component contracts | [Lily Design System — HTML Headless](https://lilydesignsystem.github.io) | 414 components | reference: `~/git/lilydesignsystem/lily-design-system/lily-design-system-html-headless/components/` |
| Visual styling | NHS UK theme (consumer-supplied for headless components) | bundled | `assets/static/css/lily.css` |

The Tera templates render the headless component contracts: each interactive element uses the canonical class name, ARIA role, and `aria-label` from the headless component spec. The bundled `lily.css` is the consumer-supplied skin; it can be swapped for any theme that styles the same class names without touching the templates.

## Per-crate layout

```
<crate>/
├── assets/
│   ├── static/
│   │   ├── css/lily.css            # Lily Design System styles (~42 KB)
│   │   └── js/
│   │       ├── htmx.min.js         # HTMX 2.0.4 (~51 KB)
│   │       └── alpine.min.js       # Alpine 3.14.8 (~45 KB)
│   └── views/
│       ├── layout.html.tera        # Base layout w/ skip-link, header, nav, footer
│       ├── home.html.tera          # Index landing page
│       ├── index.html.tera         # Entity list page
│       └── partials/
│           └── search.html.tera    # HTMX fragment for /search/partial
├── config/
│   ├── development.yaml            # Loco-conventional config
│   ├── test.yaml
│   └── production.yaml
└── src/
    ├── bin/
    │   └── web.rs                  # Runnable entry: cargo run --bin web
    └── web/
        ├── mod.rs                  # Module re-exports
        ├── app.rs                  # Loco hooks sketch + web_router() helper
        ├── views.rs                # Tera engine wrapper, WebState, AppContext
        ├── controllers.rs          # Axum handlers (home, health, audit_recent, settings, dev_error, not_found, index, show, edit, audit, export, fhir, review_queue, compare, search_page, search_partial)
        └── routes.rs               # axum::Router, ServeDir for /static, fallback to not_found
```

## URL surface

- `GET /` — home page (full HTML)
- `GET /health` — system health page (full HTML; "JSON endpoint" link wraps the existing `GET /api/health` REST endpoint)
- `GET /metrics` — performance dashboard (full HTML; supports `?range=1h|6h|24h|7d`; inline-SVG sparklines per metric + per-endpoint; wraps the existing OTLP metrics)
- `GET /tour` — guided onboarding tour (full HTML; 8 stops with per-step "Mark done" persisted in `localStorage["lily-tour-steps"]`)
- `GET /docs` — API documentation landing (full HTML; 7 collapsible endpoint groups via `accordion-nav` + verb-keyed method badges + mockup-shell curl examples; deep-links to `/swagger-ui`)
- `GET /palette` — full NHS-token color customizer (full HTML; per-token `color-picker` + `color-input`; live preview panel; overrides persist to `localStorage["lily-palette"]`)
- `GET /notifications` — persistent notification center (full HTML; 6-segment severity filter via `segment-group`; per-row mark-read / dismiss; state persists in `localStorage["lily-notifications"]`)
- `GET /audit` — system-wide recent audit activity (full HTML; supports `?action=`, `?user_id=`, `?limit=`; wraps the existing `GET /api/audit/recent` REST endpoint)
- `GET /settings` — per-browser preferences page (full HTML; settings persisted in `localStorage["lily-settings"]`)
- `GET /tokens` — personal API tokens (full HTML; paginated `data-table` of tokens with reveal/hide/copy per row via new **`secret-input`** component; 10-second auto-hide countdown with per-token Alpine timers; in-page "Generate new token" `drawer` with label + scope checkbox list + expiry select; just-generated `alert` banner displays the full secret exactly once with copy button, dismissed permanently; per-row Revoke gated by `alert-dialog`; settings page carries a "API tokens" link to this page)
- `GET /dev/500` — deliberately produce a 500 response for verifying the error page
- `GET /{entity_plural}` — entity index (full HTML; supports `?page=N`; includes bulk-select with checkbox column + bulk-action `action-bar`)
- `GET /{entity_plural}/import` — bulk-import wizard (full HTML; Alpine state, no backend round-trip in the scaffold; production posts to `POST /api/{plural}/import`)
- `GET /{entity_plural}/calendar?month=YYYY-MM` — browse records by creation date (Sun-anchored 7-column `calendar-table` grid)
- `GET /{entity_plural}/map` — schematic equirectangular world grid with 6 sample pins; production should swap for Leaflet + OSM tiles
- `GET /{entity_plural}/{id}` — entity detail page (full HTML; scaffold seeds placeholder record)
- `GET /{entity_plural}/{id}/edit` — entity edit page (full HTML; scaffold seeds record + empty errors; identifier `tag-input` + per-field `hover-card` help)
- `GET /{entity_plural}/{id}/audit` — per-record audit log (full HTML; supports `?action=`, `?page=N`)
- `GET /{entity_plural}/{id}/export` — GDPR Article 15 data export view (full HTML; "Download JSON" link wraps the existing `GET /api/{plural}/{id}/export` REST endpoint)
- `GET /{entity_plural}/{id}/fhir` — FHIR R5 resource view (full HTML; "Download JSON" link wraps the existing `GET /fhir/{ResourceType}/{id}` endpoint)
- `GET /{entity_plural}/{id}/consents` — consent management page (full HTML; lists active/revoked/expired consents; "Grant new consent" `drawer` posts to `/api/{plural}/{id}/consents`; per-row revoke posts to `/api/{plural}/{id}/consents/{cid}/revoke`)
- `GET /{entity_plural}/{id}/links` — entity-link graph view (full HTML; `tree-nav` groups Replaces / ReplacedBy / Refer / Seealso links; "Add link" `drawer` posts to `/api/{plural}/{id}/links`; per-row remove deletes via `/api/{plural}/{id}/links/{lid}`)
- `GET /{entity_plural}/{id}/qr` — QR share view (full HTML; server-rendered SVG placeholder pattern derived from a hash of the URL — real QR generation should use the `qrcode` crate; clipboard-copies the URL + Print + Download SVG)
- `GET /{entity_plural}/{id}/quality` — data-quality score view (full HTML; overall + per-component star + face ratings, prioritized improvement suggestions)
- `GET /{entity_plural}/{id}/notes` — per-record notes thread (full HTML; `chat-nav` → `chat-list` → `chat-list-item` → `chat-message` chain; role-keyed left-border accent; Alpine filter (All / Clinical / Admin / System); add-note form HTMX-posts to `/api/{plural}/{id}/notes`)
- `GET /{entity_plural}/{id}/sign?purpose=consent|witness|acknowledgement|authorisation|other` — signature capture (full HTML; introduces the new **`signature-pad`** component — HTML5 `<canvas>` with pointer / pen / touch handlers, DPR-aware on high-density screens, per-stroke `strokes[]` for one-at-a-time `Undo` + `Clear`, baseline guide and "Sign here" placeholder, `canSubmit()` gate requires drawn signature + typed name + acknowledgement checkbox; on submit emits `canvas.toDataURL("image/png")` and shows a success `alert` with a truncated `code-block` preview of the data URL; `?purpose` is server-side allowlisted and falls back to `consent`)
- `GET /{entity_plural}/review-queue` — deduplication review queue (full HTML; supports `?status=`, `?quality=`, `?page=N`)
- `GET /{entity_plural}/review-queue/kanban` — kanban-board view of the same data (full HTML; 4-column board with HTML5 drag-and-drop between Pending / Confirmed / Rejected / AutoMerged)
- `GET /{entity_plural}/deduplicate` — batch-dedup trigger wizard (full HTML; 4-step Alpine state machine config → PIN verify → running → done; uses `range-input` sliders for threshold + auto-merge-threshold, `pin-input-div` for 6-digit verification PIN, simulated progress; gates the destructive `POST /api/{plural}/deduplicate` REST endpoint behind a demo PIN `1357`)
- `GET /{entity_plural}/trash` — soft-deleted records (full HTML; paginated `data-table` of tombstone rows with bulk-select checkboxes; per-row Restore button (HTMX-posts to `/api/{plural}/{id}/restore`) + Permanently-delete button gated by per-row `alert-dialog` (HTMX-deletes via `/api/{plural}/{id}/purge`); bulk action-bar with **`dropdown-menu`** (arrow-key navigated, click-outside-closes, Esc returns focus to trigger) offering Restore selected / Permanently delete selected / Export selected as JSON; row-level `data-state="trashed"` styles trashed rows muted + italic)
- `GET /{entity_plural}/starred` — per-browser starred records (full HTML; Alpine-driven; reads `localStorage["lily-starred-{plural}"]`; server seeds 6 candidate records and `x-show`-filters to only those whose IDs are starred; empty-state `alert`; "Clear all" `alert-dialog`; the new **`star-button`** (`role="switch"`, `aria-pressed` toggle, ★ / ☆ icon, warm-yellow `data-state="on"`) is also added as a column on `/{plural}` index and as the leading action on `/{plural}/{id}` show)
- `GET /{entity_plural}/compare?a=&b=&review_id=` — side-by-side match comparison (full HTML)
- `GET /{entity_plural}/search` — full-page search (full HTML; supports `?q=`, `?fuzzy=true`, `?phonetic=true`, `?mask_sensitive=true`, `?page=N`; filter `drawer` slides in from the right)
- `GET /{entity_plural}/search/partial?q=…` — HTMX fragment (no `<html>` wrapper)
- `GET /static/*` — static assets (Lily CSS, HTMX, Alpine)
- Any unmatched path → `404` rendered by the styled `not_found.html.tera` (via `Router::fallback`)

Route ordering note: the literal segments (`/review-queue`, `/review-queue/kanban`, `/deduplicate`, `/trash`, `/starred`, `/compare`, `/import`, `/calendar`, `/map`, `/search`, `/search/partial`) are registered **before** the dynamic `/:id` so they take precedence in the matchit router. The per-record action paths (`/:id/edit`, `/:id/audit`, etc.) sit as siblings of `/:id` and disambiguate by suffix.

`{entity_plural}` is per crate: `persons`, `workers`, `places`, `things`, `events`.

## Wiring patterns

### Stand-alone Axum (current default)

`src/bin/web.rs` boots an Axum server directly:

```rust
let router = my_crate::web::router()?;
let listener = tokio::net::TcpListener::bind(addr).await?;
axum::serve(listener, router).await?;
```

### Merge into the existing REST API

The same router is mergeable with the crate's existing REST handlers:

```rust
let api = my_crate::api::rest::router();
let web = my_crate::web::router()?;
let app = api.merge(web);
```

### Loco hooks (future)

`src/web/app.rs` documents the Loco `Hooks` integration pattern. When wiring through Loco, return the web router from `Hooks::after_routes` so Loco's static middleware, config loader, and worker queue all apply.

## Templates

Templates extend `layout.html.tera`. Every template receives this context by default (see `WebState::context`):

| Variable | Type | Source |
|----------|------|--------|
| `app_display` | string | `AppContext::default().app_display` (per crate, e.g. "Person Service", "Event Service") |
| `entity_singular` | string | per crate, e.g. `"patient"` |
| `entity_plural` | string | per crate, e.g. `"persons"` |

HTMX attributes (`hx-get`, `hx-target`, `hx-trigger`, `hx-swap`) and Alpine attributes (`x-data`, `x-show`, `x-model`, `x-transition`) appear directly in markup — no client build step.

### Headless component contracts in use

Templates use the following Lily HTML Headless components. Each is rendered with the canonical class name plus the spec'd ARIA role / `aria-label`:

| Component | Tag | Class | Required ARIA | Used in |
|-----------|-----|-------|---------------|---------|
| `skip-link` | `<a>` | `skip-link` | `aria-label` | `layout` |
| `header` | `<header>` | `header` | `aria-label` | `layout` |
| `navigation-menu` | `<nav>` | `navigation-menu` | `aria-label` | `layout` |
| `footer` | `<footer>` | `footer` | `aria-label` | `layout` |
| `hero` | `<section>` | `hero` | `aria-label` | `home` |
| `form` | `<form>` | `form` | `aria-label` | `home`, `index`, `edit` |
| `field` | `<div>` | `field` | `aria-label`, optional `data-required` | `home`, `index`, `edit` |
| `label` | `<label>` | `label` | (associates via `for`) | `home`, `index`, `edit` |
| `hint` | `<span>` | `hint` | `aria-label` | `home`, `index`, `edit`, `partials/search` |
| `text-input` | `<input type="text">` | `text-input` | `aria-label`, `aria-invalid`/`aria-errormessage` when invalid | `index`, `edit` |
| `search-input` | `<input type="search">` | `search-input` | `role="searchbox"`, `aria-label` | `home` |
| `select` | `<select>` | `select` | `aria-label` | `edit` |
| `option` | `<option>` | `option` | (text) | `edit` |
| `error-summary` | `<div>` | `error-summary` | `role="alert"`, `aria-labelledby`, `tabindex="-1"` | `edit` |
| `error-message` | `<span>` | `error-message` | `role="alert"`, referenced by `aria-errormessage` | `edit` |
| `button` | `<button>` | `button` | `aria-label` | `home`, `index`, `edit`, `show` |
| `action-bar` | `<div>` | `action-bar` | `role="toolbar"`, `aria-label` | `index`, `show`, `edit` |
| `action-bar-button` | `<button>` | `action-bar-button` | `aria-label` | `show` |
| `card` | `<div>` | `card` | `role="region"`, `aria-label` | `index` |
| `alert` | `<div>` | `alert` | `role="alert"`, `aria-label`, `data-type` | `partials/search` |
| `breadcrumb-nav` | `<nav>` | `breadcrumb-nav` | `aria-label` | `show`, `edit` |
| `breadcrumb-list` | `<ol>` | `breadcrumb-list` | `aria-label` | `show`, `edit` |
| `breadcrumb-list-item` | `<li>` | `breadcrumb-list-item` | `aria-current="page"` on the leaf | `show`, `edit` |
| `summary-list` | `<ol>` | `summary-list` | `aria-label` | `show` |
| `summary-list-item` | `<li>` | `summary-list-item` | (wraps a `<dl>` with `<dt>`/`<dd>`) | `show` |
| `alert-dialog` | `<dialog>` | `alert-dialog` | `role="alertdialog"`, `aria-modal`, `aria-labelledby`, `aria-describedby` (unique-per-row IDs when many) | `show` (delete confirmation), `review_queue` (merge confirmation, one per row) |
| `data-table` | `<table>` | `data-table` | `aria-label` | `index` |
| `data-table-head` | `<thead>` | `data-table-head` | (presentational) | `index` |
| `data-table-body` | `<tbody>` | `data-table-body` | (presentational) | `index` |
| `data-table-row` | `<tr>` | `data-table-row` | (presentational) | `index` |
| `data-table-th` | `<th>` | `data-table-th` | `scope="col"` | `index` |
| `data-table-td` | `<td>` | `data-table-td` | (presentational) | `index` |
| `badge` | `<span>` | `badge` | `aria-label`, `data-type="success"`/`"error"` for status | `index` |
| `pagination-nav` | `<nav>` | `pagination-nav` | `aria-label` | `index` |
| `pagination-list` | `<ol>` | `pagination-list` | `aria-label` | `index` |
| `pagination-list-item` | `<li>` | `pagination-list-item` | (presentational) | `index` |
| `pagination-link` | `<a>` | `pagination-link` | `aria-label`; current page rendered as `<span aria-current="page">` | `index`, `review_queue` |
| `meter` | `<meter>` | `meter` | `aria-label`, `min`/`max`/`low`/`high`/`optimum`/`value`, text fallback | `review_queue` (match score) |
| `details` | `<details>` | `details` | `aria-label`, `<summary>` for the trigger | `review_queue` (score breakdown), `audit` (old/new value diff) |
| `sparkline` | `<div>` | `sparkline` | `aria-label`; wraps an inline `<svg>` `<polyline>` whose points are projected server-side by `sparkline_points(samples, w, h)` | `metrics` (system cards + per-endpoint rows) |
| `tour` | `<div>` | `tour` | `aria-label` | `tour` |
| `tour-list` | `<ol>` | `tour-list` | `aria-label` | `tour` |
| `tour-list-item` | `<li>` | `tour-list-item` | `aria-label` | `tour` (one per step; Alpine `aria-pressed` toggle marks done) |
| `qr-code` | `<div>` | `qr-code` | `aria-label`; inner `<svg>` is generated server-side from a hash of the URL plus 3 finder squares | `qr` |
| `five-star-rating-view` | `<span>` | `five-star-rating-view` | `role="img"`, `aria-label` ("N of 5 stars"); inner text is ★/☆ characters | `quality` (overall + per-component) |
| `five-face-rating-view` | `<span>` | `five-face-rating-view` | `role="img"`, `aria-label` (satisfaction descriptor); inner text is an emoji face | `quality` |
| `calendar-table` (+ head/body/foot/row/th/td) | `<table>` + thead/tbody/tfoot/tr/th/td | same | `aria-label`; weekday `<th scope="col">`; per-day `<button>` with `aria-pressed` for selection | `calendar` |
| `color-picker` | `<div>` | `color-picker` | `role="radiogroup"`, `aria-label` | `settings` (accent color) |
| `color-picker-button` | `<button>` | `color-picker-button` | `role="radio"`, `aria-label`, `aria-checked` | `settings` (per swatch) |
| `color-input` | `<input type="color">` | `color-input` | `aria-label` | `settings` (custom hex) |
| `aspect-ratio-container` | `<div>` | `aspect-ratio-container` | `aria-label`; inline `style="aspect-ratio: 2 / 1"` | `map` |
| `accordion-nav` / `accordion-list` / `accordion-list-item` | `<nav>` / `<ol>` / `<li>` | same | `aria-label`; each item wraps a `<details>` for collapsibility | `docs` |
| `command` | `<ol>` | `command` | `aria-label`; sits inside `<dialog id="command-palette">`; each `<li role="option">` has `data-active` for the keyboard-highlighted row | `layout` (palette opened via Ctrl/Cmd+K) |
| `chat-nav` / `chat-list` / `chat-list-item` / `chat-message` | `<nav>` / `<ol>` / `<li>` / `<article>` | same | `aria-label`; `chat-message` carries `data-author-role` for left-border-color accent (clinical=green, admin=orange, system=grey) | `notes` |
| `avatar` / `avatar-text` | `<div>` / `<div>` | same | `aria-label`; circular flex container with initials inside | `notes` (one per author) |
| `notification` | `<div>` | `notification` | `role="alert"`, `aria-label`, `data-severity`, `data-unread` | `notifications` (persistent center) |
| `kanban-table` (+ head/body/row/th/td) | `<table>` etc. | same | `aria-label`; one row whose `<td>`s are status-column drop targets carrying `@dragover.prevent` + `@drop.prevent` | `review_queue_kanban` |
| `segment-group` / `segment-group-item` | `<div>` / `<button>` | same | `role="group"` / `aria-pressed`-toggled exclusive selection | `notifications` (severity filter) |
| `super-banner` / `banner-box` | `<div>` / `<div>` | same | `role="alert"`, `aria-live="assertive"`, `aria-label`, `data-type` (`warning`/`critical`); per-id dismiss persists to `localStorage["lily-super-banner-dismissed-{id}"]` | `layout` (page-top announcement) |
| `text-input-with-search` | `<input type="search">` | `text-input-with-search` | `role="searchbox"`, `aria-label`; bound to the command-palette filter | `layout` |
| `mockup-browser` | `<div>` | `mockup-browser` | `aria-label`; `::before` renders the macOS-style traffic-light dots | `home` (Web UI preview card) |
| `mockup-shell` | `<div>` | `mockup-shell` | `aria-label`; `::before` renders a green `$` prompt | `home` (REST + FHIR API preview cards) |
| `mockup-phone-portrait` | `<div>` | `mockup-phone-portrait` | `aria-label`; rounded 8 px-bordered frame | `home` (Mobile preview card) |
| `action-link` | `<a>` | `action-link` | `aria-label` | `home` (mobile mockup CTA) |
| `timeline-list` | `<ol>` | `timeline-list` | `aria-label` | `audit` |
| `timeline-list-item` | `<li>` | `timeline-list-item` | (presentational; carries event header + summary-list + details) | `audit` |
| `code-block` | `<pre><code>` | `code-block` | `aria-label` | `audit` (JSON old/new values) |
| `sonner` | `<div>` | `sonner` | `role="status"`, `aria-live="polite"`, `aria-label`; one per page in `layout` as `#toast-region` | `layout` |
| `toast` | `<div>` | `toast` | `role="status"`, `aria-live="polite"`, `aria-label`, `data-type` (`success`/`info`/`warning`/`error`) | spawned by `lily.toast(message, type)` from `layout`'s inline JS, or via HTMX `HX-Trigger: {"showToast":{"message":"…","type":"…"}}` |
| `tag-group` | `<div>` | `tag-group` | `aria-label` | `search` (active filters) |
| `tag` | `<span>` | `tag` | `aria-label` | `search` (one per active filter) |
| `diff` | `<div>` | `diff` | `role="group"`, `aria-label`; two-column grid | `compare` |
| `information-callout` | `<div>` | `information-callout` | `aria-label` | `export` (GDPR notice) |
| `clipboard-copy-button` | `<button>` | `clipboard-copy-button` | `aria-label`, `data-clipboard-text` (or textContent), optional `data-copied-message` for toast text; sets `data-copied="true"` for ~2 s on success | `export` |
| `red-amber-green-view` | `<span>` | `red-amber-green-view` | `role="img"`, `aria-label`, `data-status="red"`/`"amber"`/`"green"` driving a colored CSS dot | `health` |
| `progress` | `<progress>` | `progress` | `aria-label`, `max`, `value`, text fallback | `health` (resource utilization) |
| `progress-spinner` | `<div>` | `progress-spinner` | `role="progressbar"`, `aria-label`, `aria-busy="true"`; gated behind `#htmx-indicator[data-active="true"]` | `layout` (global HTMX indicator), `import` (in-progress step) |
| `file-upload` | `<div>` | `file-upload` | `aria-label`; click + drag/drop handlers trigger `file-input.click()` | `import` |
| `file-input` | `<input type="file">` | `file-input` | `aria-label`, `accept`, `hidden`; controlled by the surrounding `file-upload` | `import` |
| `tab-bar` | `<div>` | `tab-bar` | `role="tablist"`, `aria-label` | `import` (step indicator) |
| `tab-bar-button` | `<button>` | `tab-bar-button` | `role="tab"`, `aria-label`, `aria-selected`, `tabindex` (1 selected, -1 others) | `import` |
| `checkbox-input` | `<input type="checkbox">` | `checkbox-input` | `aria-label` | `index` (per-row + select-all) |
| `tag-input` | `<div>` | `tag-input` | `aria-label`; Alpine state seeds `items` array, renders chips via `tag-group`/`tag`, hidden inputs carry data for form submit | `edit` (identifier + telecom + document management) |
| `address-input` | `<div>` | `address-input` | `aria-label`; same Alpine chip pattern as `tag-input`, but stores multi-field address objects and renders a summary line per chip | `edit` (address management) |
| `tel-input` | `<input type="tel">` | `tel-input` | `aria-label` | `edit` (telecom value when system is phone/sms/fax/pager) |
| `email-input` | `<input type="email">` | `email-input` | `aria-label` | `edit` (telecom value when system is email) |
| `kbd` | `<kbd>` | `kbd` | (text content) | `layout` (shortcuts dialog) |
| `dialog` | `<dialog>` | `dialog` | `role="dialog"`, `aria-modal="true"`, `aria-labelledby` | `layout` (shortcuts overlay) |
| `switch-button` | `<button>` | `switch-button` | `role="switch"`, `aria-label`, `aria-checked`; toggled by click + Space/Enter | `settings` |
| `hover-card` | `<div>` | `hover-card` | `role="tooltip"`, `aria-label`, `data-open` toggled by hover/focus on a sibling `.hover-card-trigger` whose `aria-describedby` references the card | `edit` (form-field help) |
| `drawer` | `<aside>` | `drawer` | `role="dialog"`, `aria-modal="true"`, `aria-label`, `data-open="true"` slides it in; backed by `.drawer-backdrop` click-catcher | `search` (filter drawer), `consents` (new-consent drawer), `links` (add-link drawer) |
| `tree-nav` | `<nav>` | `tree-nav` | `aria-label` | `links` |
| `tree-list` | `<ol>` | `tree-list` | `role="tree"`, `aria-label` | `links` |
| `tree-list-item` | `<li>` | `tree-list-item` | `role="treeitem"` | `links` |
| `tree-link` | `<a>` | `tree-link` | `aria-label` | `links` |
| `fieldset` | `<fieldset>` | `fieldset` | `aria-label`, `<legend>` for the group name | `edit` (healthcare crates only) |
| `medical-banner` | `<div>` | `medical-banner` | `role="region"`, `aria-live="polite"`, `aria-label`, `data-type`, `data-context="medical"` | `show` (healthcare crates only) |
| `medical-banner-box` | `<div>` | `medical-banner-box` | `data-context="medical"`; flex row of summary items | `show` (healthcare crates only) |
| `medical-banner-box-for-danger` | `<div>` | `medical-banner-box-for-danger` | `role="region"`, `aria-label`, `data-type="danger"` | `show` (healthcare crates only — allergies, DNR, etc.) |
| `medical-banner-box-for-advice` | `<div>` | `medical-banner-box-for-advice` | `role="region"`, `aria-label`, `data-type="advice"` | `show` (healthcare crates only — care contacts, interpreter, etc.) |
| `care-card` | `<div>` | `care-card` | `role="region"`, `aria-label` | `show` (healthcare crates only — care instructions) |
| `united-kingdom-national-health-service-number-view` | `<span>` | `united-kingdom-national-health-service-number-view` | `aria-label` (full number) | `show` (healthcare crates only) |
| `united-kingdom-national-health-service-number-input` | `<input type="text">` | `united-kingdom-national-health-service-number-input` | `aria-label`, `inputmode="numeric"`, `pattern="[0-9 ]*"`, `maxlength="12"` | `edit` (healthcare crates only) |
| `united-states-social-security-number-view` | `<span>` | `united-states-social-security-number-view` | `aria-label` (last-four only when masked) | `show` (healthcare crates only) |
| `united-states-social-security-number-input` | `<input type="text">` | `united-states-social-security-number-input` | `aria-label`, `inputmode="numeric"`, `pattern="[0-9-]*"`, `maxlength="11"`, `autocomplete="off"` | `edit` (healthcare crates only) |

### Toast bridge

The `layout.html.tera` ships an empty `#toast-region` `sonner` container plus a small inline JS bridge:

- **Direct call:** `lily.toast(message, type)` from any inline `onclick` (see the delete-confirm, merge-confirm, and reject buttons).
- **HTMX-triggered:** a controller can respond with `HX-Trigger: {"showToast":{"message":"Saved","type":"success"}}` and HTMX will dispatch a `showToast` event on `document.body`, which the bridge converts into a toast.

Both paths funnel through the same `spawn()` helper, which inserts `<div class="toast" data-type="...">` into the sonner and removes it after 5 s.

### Keyboard-shortcuts bridge

The `layout.html.tera` ships a `<dialog id="shortcuts-dialog">` listing the supported keys (rendered via `kbd` clusters in a `summary-list`) plus a `document.keydown` handler:

- `Ctrl+K` / `Cmd+K` opens the command palette (works inside inputs too).
- `?` (Shift+/) toggles the shortcuts dialog.
- `/` focuses the first `<input type="search">` or `<input type="text">` on the page.
- `g` followed by a navigation key within 1500 ms navigates: `h` → home, `i` → entity index, `s` → search, `r` → review queue, `a` → system audit, `e` → health.
- All non-`Ctrl/Cmd+K` bindings are suppressed while the user is typing into an `<input>`/`<textarea>`/`<select>`/contenteditable.

### Command palette

A `<dialog id="command-palette">` carries an Alpine-driven `command` list with 20 seeded actions (navigate to each page, switch theme, reset palette, open shortcuts). Filter as you type; arrow-keys navigate; Enter invokes; click also works. The palette dispatches three kinds of actions: `href` (`window.location.href = …`), `action: "shortcuts"` (closes itself and opens the shortcuts dialog), `action: "theme:…"` / `"palette:reset"` (mutates the relevant `localStorage` key + `<html>` attributes/styles).

### Inline-script entity caveat

Browsers parse `<script>` content as raw text — HTML entities are **not** decoded. So `&amp;&amp;` inside `<script>` is invalid JS (the literal characters `&amp;&amp;`), not the `&&` operator. Bridges in `layout.html.tera` use raw `&&` in `<script>` blocks and entity-encoded `&amp;&amp;` only inside Alpine `x-data="…"` HTML attributes (where Alpine's parser decodes them before evaluation).

### Hover-card bridge

Any `<button class="hover-card-trigger" aria-describedby="ID">…</button>` paired with a sibling `<div class="hover-card" id="ID">…</div>` becomes a focus/hover-driven tooltip. The delegated `mouseover` / `mouseout` / `focusin` / `focusout` listeners toggle `data-open="true"` on the card; `Escape` closes any open card.

### HTMX request indicator bridge

A single fixed-position `#htmx-indicator` (with `progress-spinner` + "Loading…" label) is mounted in the layout. The bridge tracks an in-flight HTMX request counter and toggles `data-active="true"` while any request is pending. `htmx:responseError` and `htmx:sendError` also fire error toasts via the toast bridge.

### Clipboard bridge

The same `layout.html.tera` block also installs a delegated click handler for `.clipboard-copy-button` elements:

- Reads `data-clipboard-text` (or falls back to the button's `textContent`).
- Calls `navigator.clipboard.writeText(...)`; sets `data-copied="true"` for ~2 s on success.
- Spawns a success toast (`data-copied-message` overrides the default "Copied to clipboard"); spawns an error toast if the browser does not expose `navigator.clipboard`.

The handler is attached to `document` so dynamically-added buttons (HTMX swaps, Alpine `x-html`, etc.) work without re-init.

### Healthcare overlay

The `show`/`edit` templates gate a healthcare overlay behind `{% if healthcare %}`. The controllers' `seed_healthcare(state)` helper returns `Some(...)` only when `state.app.entity_singular` is `"patient"` or `"person"`; for the other four crates (worker, place, thing, event) it returns `None` and the overlay is suppressed.

When present, the overlay renders:

- A `medical-banner` strip with the patient summary (`medical-banner-box`) — name, UK NHS number, US SSN, DOB
- One `medical-banner-box-for-danger` per alert (allergies, DNR, etc.)
- One `medical-banner-box-for-advice` per advice item (primary contact, interpreter, etc.)
- One `care-card` per care instruction with a quality `badge` keyed by `urgency_type`
- On the edit page, a `fieldset` with the NHS-number-input + SSN-input
- On the edit page, an additional **Emergency contacts** `fieldset` with a `tag-input`-style editor whose chips are richer `summary-list-item`s (name + relationship + Primary `badge` + per-contact `tag-group` of phone/email + Make-primary / Remove buttons). Seeded by `seed_emergency_contacts()`; gated by the same `{% if healthcare %}` block.

## Why this stack

- **No JS build pipeline.** HTMX and Alpine are loaded as static files.
- **Progressive enhancement.** Pages render fully on first GET; HTMX adds interactivity, Alpine adds local state.
- **Co-located with the Rust crate.** Templates and assets sit beside the Rust source, version-controlled together.
- **Loco-compatible.** The directory layout and YAML configs follow Loco conventions so a future `cargo loco …` workflow drops in.
- **Headless contracts, swappable theme.** Templates encode only Lily HTML Headless contracts (semantics + ARIA + class names). The visual layer (`lily.css`, currently NHS UK) can be replaced without touching templates.

## Smoke-testing locally

```bash
cd <any-crate>
cargo run --bin web      # binds 0.0.0.0:5150
curl http://127.0.0.1:5150/
curl http://127.0.0.1:5150/<plural>/search/partial?q=test
curl -I http://127.0.0.1:5150/static/css/lily.css
```
