# Web tier specification

Source of truth for the server-rendered web UI shared across the six
Main X Index crates (person / patient / worker / place / thing / event).

Each crate ships an identical `src/web/` + `assets/views/` +
`assets/static/css/lily.css` (modulo `AppContext::*()` constructor and
the runtime `entity_singular` / `entity_plural` branch points in
`seed_healthcare`, FHIR resource type, etc).

For per-crate stack and shared docs, see
[agents/share/web-stack.md](agents/share/web-stack.md). This file is
the authoritative inventory of what exists, where it lives, and what
its contract is. When templates / controllers / CSS diverge from this
file, **this file wins** — update the code to match, or update this
file with the rationale.

## Cross-crate uniformity invariant

The following files **must be byte-identical across all 6 crates**:

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
`state.app.entity_singular` (`"person"` / `"patient"` / `"worker"` /
`"place"` / `"thing"` / `"event"`) and the Tera context variables
`entity_singular` / `entity_plural` / `app_display`.

When syncing, the canonical source is `main-person-index-rust-crate/`;
copy from there to the other 5 crates.

Verify uniformity:

```bash
for f in <each shared file>; do
  hashes=$(md5 -q main-*-index-rust-crate/$f | sort -u | wc -l | tr -d ' ')
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
| `GET /dev/500` | `dev_error` | Verifies the 500 page renders |
| `GET /{entity_plural}` | `index` | Data-table + bulk-select + `?page=N` |
| `GET /{entity_plural}/import` | `import` | 5-step CSV import wizard |
| `GET /{entity_plural}/calendar` | `calendar` | Records-by-creation-date month grid (`?month=YYYY-MM`) |
| `GET /{entity_plural}/map` | `map` | Schematic equirectangular world grid with pin markers |
| `GET /{entity_plural}/review-queue` | `review_queue` | Dedup queue (`?status=`, `?quality=`, `?page=N`) |
| `GET /{entity_plural}/review-queue/kanban` | `review_queue_kanban` | Kanban board view with HTML5 drag-and-drop between status columns |
| `GET /{entity_plural}/deduplicate` | `deduplicate` | 4-step batch-dedup wizard: config (range sliders + dry-run) → PIN verify (`pin-input-div`, demo `1357`) → simulated running → results summary |
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
| `GET /static/*` | `ServeDir` | Bundled `lily.css`, `htmx.min.js`, `alpine.min.js` |
| _anything else_ | `not_found` | Styled 404 fallback (preserves the requested URI) |

`{entity_plural}` is per crate: `persons`, `patients`, `workers`,
`places`, `things`, `events`.

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

Each subsection lists: the route, the headless components on the page,
the Tera context shape, and the seeded scaffold data. "Real wiring"
notes what each handler should call once the web tier is plumbed into
`AppState`.

### `home.html.tera` — `GET /`

- **Components**: `hero`, `form` + `field` + `label` + `hint` +
  `search-input` + `button` (HTMX quick-search), `diff` (3-up grid),
  three `card`s containing `mockup-browser` / `mockup-shell` (×2) /
  `mockup-phone-portrait`, `action-link`, an endpoints `<ul>`.
- **Context**: `app_display`, `entity_singular`, `entity_plural`.
- **Real wiring**: quick-search posts to the HTMX partial; nothing else
  is dynamic.

### `index.html.tera` — `GET /{entity_plural}` (`?page=N`)

- **Components**: `action-bar` (New + Import CSV), Alpine-toggled `card`
  for the new-entity form, **bulk action-bar** with
  `action-bar-count` + bulk-delete + bulk-merge `action-bar-button`s
  (disabled unless exactly 2 selected) + `action-bar-clear` (×),
  `data-table` with select-all + per-row `checkbox-input.row-select`
  bound to `selected: []` via Alpine, status `badge`,
  `pagination-nav`, two `alert-dialog`s (bulk delete + bulk merge).
- **Context**: `records: Vec<{id, label, subtitle, active}>`,
  `pagination: {page, total_pages, has_prev, has_next, pages}`.
- **Scaffold**: 3 seeded records (2 active + 1 inactive) on 2 pages.
- **Real wiring**: replace the seeded `records` with
  `Repository::list(page, limit)`; replace `pagination` with the
  server's actual paging metadata.

### `show.html.tera` — `GET /{entity_plural}/:id`

- **Components**: `breadcrumb-nav` 3-level, healthcare overlay (gated),
  `section`, `action-bar` with 7 entries (Edit / Audit log / Export /
  FHIR / Consents / Links / Delete `action-bar-button`), `summary-list`
  with ID / Label / Subtitle / Status / Created / Updated, delete
  `alert-dialog` whose confirm button HTMX-deletes and spawns a toast.
- **Context**: `record: {id, label, subtitle, active, created_at,
  updated_at}`, optional `healthcare: { … }`.
- **Real wiring**: `Repository::get_by_id(id)` → 404 via `not_found`
  if missing.

### `edit.html.tera` — `GET /{entity_plural}/:id/edit`

The most complex page. 4 list editors plus base fields.

- **Header**: `breadcrumb-nav` 4-level, optional `error-summary`
  (`role="alert"`, `tabindex="-1"`) rendered only when
  `errors | length > 0`.
- **Form** (HTMX `hx-put="/api/{plural}/{id}"`, `novalidate`):
  - 3 base `field`s: Label (`text-input`, `data-required`, hover-card help),
    Subtitle (`text-input`, hover-card help), Status (`select` /
    `option` × 2, hover-card help).
  - **Healthcare identifiers fieldset** — gated by `{% if healthcare %}`,
    contains `united-kingdom-national-health-service-number-input` +
    `united-states-social-security-number-input`. Patient + person only.
  - **Identifiers fieldset** — `tag-input` editor, Alpine `items`
    seeded from `identifiers`. 7-option type `select` (MRN / SSN / NPI
    / PPN / DL / TAX / OTHER) + `text-input` value, hidden inputs
    serialize to `identifiers[i][type]` / `identifiers[i][value]`.
  - **Addresses fieldset** — `address-input` wrapper, chips show
    flattened summary line, 6 inline fields (Line 1, Line 2, City,
    State, Postal code, Country), serialize to
    `addresses[i][field]`.
  - **Contacts fieldset** — `tag-input`, 6-option system `select`
    (phone / email / sms / fax / pager / url); the value input swaps
    type per system via Alpine `x-if`: `tel-input` for phone / sms /
    fax / pager, `email-input` for email, `text-input[type=url]` for
    url. Serializes to `telecoms[i][system]` / `telecoms[i][value]`.
  - **Emergency contacts fieldset** — gated by `{% if healthcare %}`.
    Chips are `summary-list-item`s (richer than plain `tag`) with
    name + relationship + Primary `badge` + per-contact `tag-group`
    of phone / email + per-row Make-primary / Remove `action-bar-button`s.
    Add form: name, 8-option relationship `select` (spouse / partner /
    parent / child / sibling / guardian / friend / other), `tel-input`
    phone, `email-input` email, primary checkbox. Validation
    (`.error-message[role="alert"]`): name required, relationship
    required, at least one channel required. Setting `is_primary`
    clears that flag on all other rows.
  - **Identity documents fieldset** — `tag-input`, 9-option type
    `select` (PASSPORT / BIRTH_CERTIFICATE / NATIONAL_ID /
    DRIVERS_LICENSE / VOTER_ID / MILITARY_ID / RESIDENCE_PERMIT /
    WORK_PERMIT / OTHER), required number `text-input`, ISO country
    (maxlength=2), authority, `<input type="date">` issue + expiry.
    Per-chip freshness `badge` keyed by `expiryBadge(item)`:
    `success` valid, `warning` "expiring soon" (<6 months),
    `error` expired, `info` no expiry. Validation: number required,
    `issue_date < expiry_date` if both present.
- **Footer**: `action-bar` with Save (button) + Cancel (link to show
  page).
- **Context**: `record`, `errors: Vec<_>`, `field_errors:
  HashMap<String, String>`, `identifiers`, `addresses`, `telecoms`,
  `documents`, optional `healthcare`, optional `emergency_contacts`.
- **Real wiring**: pre-populate from the repository; on PUT failure,
  re-render with the 422 `field_errors` and `errors` list populated.

### `audit.html.tera` — `GET /{entity_plural}/:id/audit`

- **Components**: `breadcrumb-nav`, `form` + `select` action filter
  (HTMX `hx-trigger="change"`), `timeline-list` of
  `timeline-list-item`s, action-keyed `badge` (Created=success,
  Updated=info, Deleted=error, Merged=warning), `<time>`,
  `summary-list` per entry (User + IP / user-agent / summary),
  collapsible `<details>` + `code-block` for old/new value diff,
  `pagination-nav`.
- **Context**: `record`, `entries`, `filter_action`, `pagination`.
- **Scaffold**: 6 entries covering Created / Updated×2 / Merged /
  Linked / Deleted.
- **Real wiring**: `AuditLogRepository::get_history(entity_id)`.

### `export.html.tera` — `GET /{entity_plural}/:id/export`

- **Components**: `breadcrumb-nav`, `information-callout` (GDPR
  Article 15 notice), `action-bar` (Download JSON `<a download>` →
  `/api/{plural}/{id}/export`, `clipboard-copy-button` with
  `data-clipboard-text` + custom `data-copied-message`, Back),
  `summary-list` (record ID / generated at / byte size / active
  consents as quality-keyed `badge`s), `code-block` with
  pretty-printed JSON payload.
- **Context**: `record`, `export_json` (compact), `export_json_pretty`,
  `export_size_bytes`, `generated_at`, `consents`.
- **Real wiring**: `crate::privacy::export_person_data(person)` (or
  patient equivalent).

### `fhir.html.tera` — `GET /{entity_plural}/:id/fhir`

- **Components**: `breadcrumb-nav`, `information-callout` (FHIR R5
  notice), `action-bar` (Download JSON → `/fhir/{ResourceType}/{id}`,
  `clipboard-copy-button`, GDPR export, Back), `summary-list`
  (resource type / FHIR ID / version / generated-at /
  `application/fhir+json`), `code-block`.
- **Context**: `record`, `fhir_resource_type`, `fhir_json`,
  `fhir_json_pretty`, `generated_at`.
- **Resource-type mapping**: patient → `Patient`, person → `Person`,
  worker → `Practitioner`, place → `Location`, thing → `Device`,
  event → `Observation`, default → `Basic`.
- **Real wiring**: `crate::api::fhir::resources::to_fhir(person)`.

### `consents.html.tera` — `GET /{entity_plural}/:id/consents`

- **Components**: `breadcrumb-nav`, `information-callout` (GDPR
  Article 7 / HIPAA explainer), `action-bar` with stats line +
  "Grant new consent" button → opens `drawer`, `data-table`
  (Type / Status / Granted / Expires / Method / Purpose / Actions),
  status `badge` (Active=success, Revoked=error, Expired=warning),
  per-row Revoke `action-bar-button` (Active only) → per-row
  `alert-dialog` (unique IDs per row) HTMX-posts revoke + toast,
  `drawer` "Grant new consent" with `form` + 5-option type `select`
  + 4-option method `select` + purpose `text-input` +
  `<input type="date">` expiry.
- **Context**: `record`, `consents`, `stats: {active, revoked, expired}`.
- **Real wiring**: list / grant / revoke against the consents
  repository.

### `links.html.tera` — `GET /{entity_plural}/:id/links`

- **Components**: `breadcrumb-nav`, `information-callout`, `action-bar`
  (total count + Add-link `drawer`), `tree-nav` grouping the 4 link
  kinds in `<details open>` blocks, each with a kind-keyed `badge`
  (Replaces=warning, ReplacedBy=error, Refer=info, Seealso=success),
  `tree-list` (`role="tree"`) of `tree-list-item`s
  (`role="treeitem"`) containing a `tree-link` + optional merge-score
  hint + optional note + per-row Remove `action-bar-button`
  (disabled via Alpine for merge-derived `Replaces`/`ReplacedBy`),
  drawer "Add link" with 2-option link-type `select` (Refer /
  Seealso only — merge-derived links are auto), target UUID
  `text-input`, optional note.
- **Context**: `record`, `groups: Vec<{kind, description, items}>`,
  `totals: {total}`.
- **Real wiring**: `PersonLink` (or `PatientLink`) joined against
  target entities for labels.

### `review_queue.html.tera` — `GET /{entity_plural}/review-queue`

- **Components**: `breadcrumb-nav`, header stats hint, two `select`
  filters (status + match-quality) with HTMX `hx-trigger="change"`,
  `tag-group` of active-filter chips, `data-table` (Candidate A,
  Candidate B, Score with `meter` + numeric + collapsible
  `<details>` + `summary-list` breakdown, Quality `badge`, Detected,
  Actions: Compare link / Merge button → per-row `alert-dialog`
  (unique IDs) / Reject `action-bar-button` HTMX-posting), empty-state
  `alert`, `pagination-nav`.
- **Context**: `items: Vec<{id, a_id, a_label, b_id, b_label, score,
  score_pct, quality, detection_method, created_at, breakdown}>`,
  `stats`, `pagination`, `filter_status`, `filter_quality`.
- **Real wiring**: `ReviewQueueRepository::list(status, quality,
  page)` + the existing `/api/{plural}/merge` and
  `/api/{plural}/review-queue/{id}/reject` REST endpoints.

### `deduplicate.html.tera` — `GET /{entity_plural}/deduplicate`

- **Components**: `breadcrumb-nav`, `information-callout` (explainer
  with demo-PIN disclosure), 4 `<section>` blocks Alpine-gated by
  `step ∈ {"config","verify","running","done"}`. Step 1 = `form` with
  two `range-input` sliders (threshold 0.50–0.99, auto-merge 0.85–1.00),
  numeric `text-input` for max-candidates, dry-run checkbox, `action-bar`
  (Continue / Cancel). Step 2 = `pin-input-div` wrapping 6 single-digit
  `text-input` boxes (template-iterated) with auto-advance on numeric
  keypress + back-step on Backspace via querySelector-based focus
  helper (Alpine has no dynamic `x-ref`), Verify / Back actions. Step 3
  = `progress` bar + `progress-spinner` + dry-run hint. Step 4 = success
  `alert` + `summary-list` of `scanned / duplicates / auto_merged /
  queued / ms` badges + Open-review-queue link + Run-another button.
- **State**: `x-data` holds `threshold`, `autoMergeThreshold`,
  `maxCandidates`, `dryRun`, `pin[6]`, `step`, `progressPercent`,
  `result`. Demo PIN `1357`; rejection raises `lily.toast(…, "error")`.
- **Context**: page is currently scaffold-only (no server-side seed
  beyond the standard `state.context()`).
- **Real wiring**: gates the existing destructive `POST
  /api/{plural}/deduplicate` REST endpoint behind PIN verification
  (production should `POST /api/auth/verify-pin` server-side rather
  than checking the demo constant client-side).

### `compare.html.tera` — `GET /{entity_plural}/compare?a=&b=&review_id=`

- **Components**: `breadcrumb-nav` 4-level, overall-match `card`
  with `meter` + quality `badge` + detection-method `hint`,
  `diff` 2-column grid containing two `card` panels each with a
  `summary-list` of the candidate's fields, `data-table` of per-component
  scores with `meter` bars + outcome `badge` (match=success / close=warning
  / differ=error), `action-bar` (Merge button → `alert-dialog`,
  Not-a-duplicate `action-bar-button`, Back to queue), merge
  `alert-dialog`.
- **Context**: `a: {id, label}`, `b: {id, label}`, `fields:
  Vec<{label, a_value, b_value, score, score_pct, outcome}>`,
  `overall: {score, score_pct, quality, detection_method}`,
  `review_id`.
- **Real wiring**: match against the live records via
  `MatcherProbabilistic::match_persons(a, b)`.

### `search.html.tera` — `GET /{entity_plural}/search`

- **Components**: `breadcrumb-nav`, `form` + `search-input` + 3 filter
  checkbox `field`s (fuzzy / phonetic / mask sensitive) inside a
  slide-out `drawer` (Alpine `drawerOpen`), `drawer-backdrop`
  click-catcher, action-bar with Search / Reset / Filters,
  `tag-group` of active filters, `data-table` (Label / Subtitle /
  Status / Relevance `meter`), `pagination-nav`, empty-state `alert`.
- **Context**: `query`, `fuzzy`, `phonetic`, `mask_sensitive`,
  `active_filters`, `results`, `result_total`, `pagination`.
- **Real wiring**: `SearchEngine::search(...)`.

### `search.html.tera` partial — `GET /{entity_plural}/search/partial?q=`

- HTMX fragment, no `<html>` wrapper. Returns matching results in a
  format suitable for HTMX swap (currently a `<ul>` plus
  alert / hint when empty).

### `audit_recent.html.tera` — `GET /audit`

- System-wide audit timeline. Same component vocabulary as the
  per-record `audit.html.tera` but with per-entry deep links to
  `/{plural}/{entity_id}/audit`.
- **Context**: `entries`, `filter_action`, `filter_user`,
  `filter_limit`, `active_filters`.

### `health.html.tera` — `GET /health`

- **Components**: `breadcrumb-nav`, overall-status `card` with
  `red-amber-green-view` + status `badge` + `summary-list` (service /
  version / uptime / checked-at) + Refresh (HTMX) + JSON-endpoint
  buttons, subsystems `data-table` (Subsystem / RAG / Latency
  `meter` / Detail), resource-utilization `diff` grid of 4 `card`s
  each with a `<progress>` bar + quality `badge`, recent-incidents
  `timeline-list` with severity-keyed `badge`s.
- **Context**: `service`, `overall`, `subsystems`, `resources`,
  `recent_incidents`.
- **Version**: `env!("CARGO_PKG_VERSION")` — accurate per crate.

### `metrics.html.tera` — `GET /metrics?range=1h|6h|24h|7d`

- **Components**: `breadcrumb-nav`, time-range `select` (HTMX),
  system-overview `diff` grid of 4 `card`s each with a `sparkline`
  (inline SVG `<polyline>`) + trend `badge` + min/max hint,
  endpoint-latency `data-table` with per-row `red-amber-green-view`
  + p50/p95 `meter`/p99/req-sec + per-endpoint `sparkline`,
  errors-by-class `summary-list` with HTTP-class `badge`s.
- **Context**: `range`, `range_label`, `system_metrics`, `endpoints`,
  `error_classes`.
- **Sparkline projection**: `sparkline_points(samples, w, h)` projects
  a `&[u32]` series onto an SVG viewBox (Y inverted), returns a
  `"x,y x,y …"` string for `<polyline points="…">`. Auto-scales by
  min/max; handles empty/constant arrays.

### `settings.html.tera` — `GET /settings`

- Pure client-side preferences page; no server round-trip. Alpine
  reads `localStorage["lily-settings"]` on load and writes on save.
- **Components**: `breadcrumb-nav`, `form` with 2 fieldsets
  (Display, Diagnostics), 4 `switch-button`s (`role="switch"`,
  `aria-checked` toggle): mask sensitive default, show inactive,
  toasts on, show technical detail; `select` page-size; Save / Reset
  action-bar.

### `tour.html.tera` — `GET /tour`

- **Components**: `breadcrumb-nav`, `information-callout`, progress
  `action-bar` (count + `<progress>` + Reset), `tour` wrapper →
  `tour-list` of 8 `tour-list-item`s. Each step: numbered/checkmark
  `badge` (`success` when done, `info` otherwise via Alpine
  `isDone(key)`), title, optional `kbd` cluster, body HTML (allows
  `<strong>`, `<code>`, `<kbd>`), per-step action-bar with "Open" CTA
  (deep-link) + "Mark done" toggle (`aria-pressed`).
- **Persistence**: `localStorage["lily-tour-steps"]` array of done
  keys.

### `import.html.tera` — `GET /{entity_plural}/import`

5-step CSV wizard, pure Alpine. No server round-trip.

- **Components**: `breadcrumb-nav`, `information-callout`, `tab-bar`
  step indicator (5 `tab-bar-button`s with dynamic
  `aria-selected` / `tabindex` / `disabled`):
  1. Upload: `file-upload` drop zone + hidden `file-input`, parses
     CSV via `FileReader`, advances to step 1
  2. Preview: dynamic `data-table` of first 5 sample rows
  3. Map columns: per-column `select` mapping to entity fields
  4. Importing: `<progress>` + `progress-spinner`, 200 ms tick to
     100 %
  5. Done: success `alert`, `summary-list` of 4 result `badge`s
     (Inserted=success, Updated=info, Skipped=warning, Errors)
- **Real wiring**: streamed multipart POST to
  `/api/{plural}/import` with SSE / HTMX `HX-Trigger:
  importProgress` channel for live progress.

### `not_found.html.tera` — fallback for unmatched routes

- **Components**: `alert[data-type="warning"]` echoing the requested
  path (extracted via Axum's `Uri`), `summary-list` of useful
  destinations, `back-link`.

### `error.html.tera` — styled 5xx page

- **Components**: `alert[data-type="error"]`, optional message,
  `summary-list` (request_id + at), collapsible `<details>` +
  `code-block` for optional technical trace, action-bar (Home +
  System health), `back-link`.
- **Helper**: `pub fn error_page(state, status, message, detail) ->
  Response` — call from any handler that needs a styled 5xx.

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

Every Lily HTML Headless component used by the web tier, in
alphabetical order, with the canonical class / tag / required ARIA and
where it appears.

| Component | Tag | Class | Required ARIA | Used in |
|---|---|---|---|---|
| `action-bar` | `<div>` | `action-bar` | `role="toolbar"`, `aria-label` | `index` (× 2), `show`, `edit`, `review_queue`, `compare`, `consents`, `links`, `health`, `metrics`, `search`, `import`, `audit`, `tour`, `home`, `error`, `not_found` |
| `action-bar-button` | `<button>` | `action-bar-button` | `aria-label` | `show` (delete), `review_queue` (reject), `consents` (revoke), `links` (remove) |
| `action-link` | `<a>` | `action-link` | `aria-label` | `home` (mobile mockup CTA) |
| `address-input` | `<div>` | `address-input` | `aria-label` | `edit` |
| `alert` | `<div>` | `alert` | `role="alert"`, `aria-label`, `data-type` | `partials/search`, `import`, `not_found`, `error`, `search`, `audit`, `audit_recent` |
| `alert-dialog` | `<dialog>` | `alert-dialog` | `role="alertdialog"`, `aria-modal`, `aria-labelledby`, `aria-describedby` (unique IDs when many) | `show` (delete), `review_queue` (merge per row), `compare` (merge), `consents` (revoke per row), `index` (bulk delete + bulk merge) |
| `badge` | `<span>` | `badge` | `aria-label`, `data-type` ∈ `success`/`info`/`warning`/`error` | almost every page |
| `breadcrumb-nav` / `breadcrumb-list` / `breadcrumb-list-item` | `<nav>` / `<ol>` / `<li>` | same | `aria-label`; `aria-current="page"` on leaf | every non-home page |
| `button` | `<button>` | `button` | `aria-label` | almost every page |
| `card` | `<div>` | `card` | `role="region"`, `aria-label` | `index`, `home`, `metrics`, `health`, `compare`, `tour` (implicit via `tour-list-item`) |
| `checkbox-input` | `<input type="checkbox">` | `checkbox-input` | `aria-label` | `index` (bulk-select) |
| `clipboard-copy-button` | `<button>` | `clipboard-copy-button` | `aria-label`, `data-clipboard-text`, optional `data-copied-message` | `export`, `fhir` |
| `code-block` | `<pre><code>` | `code-block` | `aria-label` | `audit` (old/new value diff), `export`, `fhir`, `error` |
| `data-table` (+ head/body/row/th/td) | `<table>` etc. | same | `aria-label`; `scope="col"` on `<th>` | `index`, `review_queue`, `compare`, `audit`, `search`, `metrics`, `consents`, `import` |
| `details` | `<details>` | `details` | `aria-label`, `<summary>` | `audit` (value diff), `review_queue` (score breakdown), `error`, `links` (group), `tour` (could) |
| `diff` | `<div>` | `diff` | `role="group"`, `aria-label`; CSS grid 2-col → 1-col < 768 px | `compare`, `health`, `metrics`, `home` |
| `dialog` | `<dialog>` | `dialog` | `role="dialog"`, `aria-modal`, `aria-labelledby` | `layout` (shortcuts) |
| `drawer` | `<aside>` | `drawer` | `role="dialog"`, `aria-modal`, `aria-label`, `data-open` | `search`, `consents`, `links` |
| `email-input` | `<input type="email">` | `email-input` | `aria-label` | `edit` (telecom + emergency contact) |
| `error-message` | `<span>` | `error-message` | `role="alert"` | `edit` (per field), `edit` (validation in tag-inputs) |
| `error-summary` | `<div>` | `error-summary` | `role="alert"`, `aria-labelledby`, `tabindex="-1"` | `edit` |
| `field` | `<div>` | `field` | `aria-label`, optional `data-required` | `home`, `index`, `edit`, `search`, `settings`, `consents`, `links`, `import`, `metrics`, `audit`, `audit_recent` |
| `fieldset` | `<fieldset>` | `fieldset` | `aria-label`, `<legend>` | `edit` (identifiers, addresses, contacts, documents, emergency contacts, healthcare), `settings` |
| `file-input` | `<input type="file">` | `file-input` | `aria-label`, `accept`, `hidden` | `import` |
| `file-upload` | `<div>` | `file-upload` | `aria-label`; click + drag/drop handlers | `import` |
| `footer` | `<footer>` | `footer` | `aria-label` | `layout` |
| `form` | `<form>` | `form` | `aria-label` | `home`, `index`, `edit`, `search`, `consents`, `links`, `audit`, `audit_recent`, `metrics`, `import`, `settings`, `tour` (could) |
| `header` | `<header>` | `header` | `aria-label` | `layout`, plus per-page sections via `<header>` |
| `hero` | `<section>` | `hero` | `aria-label` | `home` |
| `hint` | `<span>` | `hint` | `aria-label` | almost every page |
| `hover-card` | `<div>` | `hover-card` | `role="tooltip"`, `aria-label`, `data-open` driven by sibling `.hover-card-trigger[aria-describedby]` | `edit` (Label / Subtitle / Status) |
| `information-callout` | `<div>` | `information-callout` | `aria-label` | `export`, `fhir`, `consents`, `links`, `import`, `tour` |
| `kbd` | `<kbd>` | `kbd` | (text content) | `layout` (shortcuts dialog), `tour` |
| `label` | `<label>` | `label` | (`for` association) | every form |
| `medical-banner` / `-box` / `-box-for-danger` / `-box-for-advice` | `<div>` | same | `role="region"`, `aria-live="polite"`, `aria-label`, `data-type`, `data-context="medical"` | `show` (healthcare crates only) |
| `care-card` | `<div>` | `care-card` | `role="region"`, `aria-label` | `show` (healthcare crates only) |
| `meter` | `<meter>` | `meter` | `aria-label`, `min`/`max`/`low`/`high`/`optimum`/`value` | `review_queue`, `compare`, `search`, `metrics`, `health` |
| `mockup-browser` / `mockup-shell` / `mockup-phone-portrait` | `<div>` | same | `aria-label`; CSS `::before` for browser dots / shell `$` prompt | `home` |
| `navigation-menu` | `<nav>` | `navigation-menu` | `aria-label` | `layout` |
| `option` | `<option>` | `option` | (text) | every `select` |
| `pagination-nav` / `-list` / `-list-item` / `-link` | `<nav>` / `<ol>` / `<li>` / `<a>` | same | `aria-label`; current page is `<span aria-current="page">` | `index`, `review_queue`, `search`, `audit`, `consents` |
| `pin-input-div` | `<div>` wrapping N `<input>` | `pin-input-div` | `aria-label` on container; per-digit `aria-label="PIN digit i of N"`, `inputmode="numeric"`, `maxlength="1"`, `pattern="[0-9]"` | `deduplicate` (6-digit verify step) |
| `progress` | `<progress>` | `progress` | `aria-label`, `max`, `value` | `health` (utilization), `import` (in-progress), `tour` (completion), `deduplicate` (scan progress) |
| `progress-spinner` | `<div>` | `progress-spinner` | `role="progressbar"`, `aria-label`, `aria-busy="true"` | `layout` (HTMX indicator), `import`, `deduplicate` |
| `range-input` | `<input type="range">` | `range-input` | `aria-label`, `aria-describedby`, `min`, `max`, `step` | `deduplicate` (threshold + auto-merge-threshold sliders) |
| `red-amber-green-view` | `<span>` | `red-amber-green-view` | `role="img"`, `aria-label`, `data-status="red"`/`"amber"`/`"green"` | `health`, `metrics` |
| `search-input` | `<input type="search">` | `search-input` | `role="searchbox"`, `aria-label` | `home`, `search` |
| `select` | `<select>` | `select` | `aria-label` | every page with a dropdown |
| `skip-link` | `<a>` | `skip-link` | `aria-label`, `href="#main-content"` | `layout` |
| `sonner` | `<div>` | `sonner` | `role="status"`, `aria-live="polite"`, `aria-label` | `layout` (`#toast-region`) |
| `sparkline` | `<div>` | `sparkline` | `aria-label`; wraps inline-SVG `<polyline>` | `metrics` |
| `summary-list` / `summary-list-item` | `<ol>` / `<li>` | same | `aria-label` | `show`, `export`, `fhir`, `health`, `audit`, `audit_recent`, `metrics`, `review_queue`, `not_found`, `home`, `edit` (emergency contacts), `tour`, `import` |
| `switch-button` | `<button>` | `switch-button` | `role="switch"`, `aria-label`, `aria-checked` | `settings` |
| `tab-bar` / `tab-bar-button` | `<div>` / `<button>` | same | `role="tablist"`, `role="tab"`, `aria-selected`, `tabindex` | `import` (step indicator) |
| `tag-group` / `tag` | `<div>` / `<span>` | same | `aria-label` | `search` (active filters), `edit` (chip pattern), `consents` (per-row badge cluster) |
| `tag-input` | `<div>` | `tag-input` | `aria-label` | `edit` (identifiers, telecoms, documents, emergency contacts) |
| `tel-input` | `<input type="tel">` | `tel-input` | `aria-label` | `edit` (telecom phone / sms / fax / pager + emergency contact phone) |
| `text-input` | `<input type="text">` | `text-input` | `aria-label`, optional `aria-invalid` / `aria-errormessage` | every form |
| `theme-select` / `theme-select-option` | `<select>` / `<option>` | same | `aria-label` | `layout` |
| `timeline-list` / `timeline-list-item` | `<ol>` / `<li>` | same | `aria-label` | `audit`, `audit_recent`, `health` (recent incidents) |
| `toast` | `<div>` | `toast` | `role="status"`, `aria-live="polite"`, `aria-label`, `data-type` | spawned dynamically by `lily.toast(…)` |
| `tour` / `tour-list` / `tour-list-item` | `<div>` / `<ol>` / `<li>` | same | `aria-label` | `tour` |
| `tree-nav` / `tree-list` / `tree-list-item` / `tree-link` | `<nav>` / `<ol>` / `<li>` / `<a>` | same | `aria-label`, `role="tree"`, `role="treeitem"` | `links` |
| `united-kingdom-national-health-service-number-input` / `-view` | `<input type="text">` / `<span>` | same | `aria-label`, `inputmode="numeric"`, `pattern="[0-9 ]*"`, `maxlength="12"` (input) | `edit` / `show` (healthcare crates only) |
| `united-states-social-security-number-input` / `-view` | `<input type="text">` / `<span>` | same | `aria-label`, `inputmode="numeric"`, `pattern="[0-9-]*"`, `maxlength="11"`, `autocomplete="off"` (input) | `edit` / `show` (healthcare crates only) |

## Healthcare overlay

The `show` and `edit` templates each gate a healthcare overlay behind
`{% if healthcare %}`. The Rust `seed_healthcare(state)` helper returns
`Some(...)` only when `state.app.entity_singular ∈ {"patient",
"person"}`; otherwise `None` (and `emergency_contacts` defaults to an
empty `Vec`).

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

## Conventions

### Per-crate uniformity

When adding a new view: build it in
`main-person-index-rust-crate/`, smoke-test, then sync to the other 5
crates with the standard pattern:

```bash
SRC=/Users/jph/git/sixarm/main-x-index/main-person-index-rust-crate
for d in main-event-index-rust-crate main-patient-index-rust-crate \
         main-place-index-rust-crate main-thing-index-rust-crate \
         main-worker-index-rust-crate; do
  DST=/Users/jph/git/sixarm/main-x-index/$d
  cp "$SRC/<file>" "$DST/<file>"
done
```

Then `cargo check --bin web` in each crate (run in parallel via
background tasks).

### Per-crate runtime branching

Branching on entity type happens at the Rust level via
`state.app.entity_singular`. Examples:

- `seed_healthcare(state)` — `Some(...)` only for `"patient"` /
  `"person"`
- `fhir(state, id)` — maps `entity_singular` to FHIR resource type
- `tour(state)` — builds per-crate `href` values

Templates never `{% if entity_singular == "patient" %}`; they branch
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
`PersonRepository` / `PatientRepository` / `SearchEngine` /
`AuditLogRepository` / event publisher) into `WebState` and then
replace the `seed_*` calls in each handler.

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
