# Web pages — per-page contracts

This file is the per-page contract reference for the server-rendered
web UI shared across the five service crates (person / worker / place /
thing / event). For the cross-cutting URL surface, layout, JS bridges,
CSS conventions, data attributes, and localStorage keys, see the
project-root [`spec.md`](../../spec.md). For the headless component
inventory, see [`web-stack.md`](web-stack.md).

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
- **Real wiring**: per-crate `crate::privacy::export_*_data(record)`
  (e.g. `export_person_data`, `export_event_data`).

### `fhir.html.tera` — `GET /{entity_plural}/:id/fhir`

- **Components**: `breadcrumb-nav`, `information-callout` (FHIR R5
  notice), `action-bar` (Download JSON → `/fhir/{ResourceType}/{id}`,
  `clipboard-copy-button`, GDPR export, Back), `summary-list`
  (resource type / FHIR ID / version / generated-at /
  `application/fhir+json`), `code-block`.
- **Context**: `record`, `fhir_resource_type`, `fhir_json`,
  `fhir_json_pretty`, `generated_at`.
- **Resource-type mapping**: person → `Person`, worker →
  `Practitioner`, place → `Location`, thing → `Device`, event →
  `Observation`, default → `Basic`.
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

### `sign.html.tera` — `GET /{entity_plural}/:id/sign?purpose=…`

- **Components**: `breadcrumb-nav` (4-level: Home / {plural} / {record}
  / Sign), `information-callout` (explains the wire payload), `form`
  with two `fieldset`s — Context (`select` purpose + `text-input` name
  + `datetime-local` signed-at) and Signature (the new
  **`signature-pad`** + per-canvas `action-bar` with Undo / Clear),
  acknowledgement `checkbox-input`, submit `action-bar`, on-submit
  success `alert` with `details` + `code-block` showing a truncated
  data-URL preview.
- **`signature-pad` contract**: `<div class="signature-pad">` carries
  `aria-label` + dynamic `data-state="empty"` / `"drawn"`. Inside: an
  `<canvas class="signature-pad-canvas">` with `role="img"` +
  `aria-label` + `style="touch-action: none;"` (the CSS rule defers
  scroll/zoom gestures to the drawing logic), a dashed
  `signature-pad-baseline` and an italic `signature-pad-placeholder`
  ("Sign here") both `aria-hidden="true"`. The pad listens for
  `pointerdown` / `pointermove` / `pointerup` / `pointerleave` /
  `pointercancel`, uses `canvas.setPointerCapture(pointerId)` for
  finger/pen-off-edge tolerance, and stores per-stroke arrays so
  `Undo` can pop one stroke and `redraw()` the rest. The canvas is
  resized to `devicePixelRatio` and the 2D context scaled to match,
  so high-DPI screens get crisp strokes. The toolbar buttons disable
  themselves when `!hasSignature()`.
- **State**: `purpose`, `signedByName`, `signedAt` (defaults to
  `new Date().toISOString().slice(0, 16)`), `acknowledged`,
  `strokes: Vec<Vec<{x, y}>>`, `current` (in-progress stroke),
  `dataUrl` (PNG base64 produced on submit), `saved` (banner
  visibility), `canSubmit()` (`hasSignature && signedByName &&
  acknowledged`).
- **Context**: `record: {id, label, ...}`, `purpose: String`. The
  query `?purpose` is server-side allowlisted (`consent`, `witness`,
  `acknowledgement`, `authorisation`, `other`) and defaults to
  `consent` on missing or invalid input — this avoids stored-XSS risk
  from inserting unescaped user-provided text into the page.
- **Real wiring**: `POST /api/{plural}/{id}/signatures` with
  `{ purpose, signed_by_name, signed_at, image_png_base64,
  audit_user_ip, audit_user_agent }`. The signature record should
  append to the audit log and (for `purpose=consent`) update the
  related `Consent` row's `granted_signature_id`. The show page's
  action-bar carries a "Sign" button to `?purpose=consent`.

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
- **Real wiring**: the per-crate `*Link` model (e.g. `PersonLink`,
  `EventLink`) joined against target entities for labels.

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

### `starred.html.tera` — `GET /{entity_plural}/starred`

- **Components**: `breadcrumb-nav` (Home / {plural} / Starred),
  summary `hint` (with live count from `visibleCount()`), top
  `action-bar` ("← All {plural}" link + "Clear all"
  `action-bar-button` gated by `alert-dialog`, only visible when
  `starred.length > 0`), empty-state `alert` (only visible when
  `starred.length === 0`, with a tiny inline ☆ icon to demonstrate
  the affordance), `data-table` (only visible when
  `starred.length > 0`) with star toggle column + Label + Subtitle +
  Status, per-row `x-show="isStarred(id)"` to hide unstarred rows
  with `x-transition`, "Clear all" `alert-dialog`.
- **State**: `x-data` holds `starred: Vec<String>` (seeded from
  `localStorage["lily-starred-{plural}"]`), `isStarred(id)`,
  `toggleStar(id, label)` (mutates + persists + toasts),
  `clearAll()` (empties + removes the key + toasts),
  `visibleCount()`.
- **Context**: `candidates: Vec<{id, label, subtitle, active}>`.
  Scaffold seeds 6 records; production should send all records the
  user has permission to view and let the page filter, or accept
  a `?ids=…` query parameter for a server-side pre-filter.
- **Cross-page coupling**: the same `star-button` (with identical
  Alpine helpers `isStarred` / `toggleStar`) appears as a column on
  `index.html.tera` and as the leading button in `show.html.tera`'s
  action-bar. All three pages read and write the same
  `localStorage["lily-starred-{plural}"]` key, so toggles propagate
  across tabs on next render.

### `trash.html.tera` — `GET /{entity_plural}/trash`

- **Components**: `breadcrumb-nav`, summary `hint`, top `action-bar`
  with "← Active {plural}" link, empty-state `alert`, bulk
  `action-bar` (visible when `selected.length > 0`) hosting a new
  **`dropdown-menu`** (Restore selected / Permanently delete selected
  / Export selected as JSON), bulk-purge `alert-dialog`, `data-table`
  with select-all checkbox + per-row checkbox + Label + Soft-deleted
  at + Deleted by + Reason + Actions (Restore button HTMX-posting to
  `/api/{plural}/{id}/restore`; Delete-forever button gated by per-row
  `alert-dialog` HTMX-deleting via `/api/{plural}/{id}/purge`),
  `pagination-nav`. Trashed rows carry `data-state="trashed"` for
  muted + italic styling.
- **`dropdown-menu` contract**: trigger button with
  `aria-haspopup="menu"` + dynamic `aria-expanded`; menu list with
  `role="menu"`; items with `role="menuitem"` + `tabindex="-1"`;
  destructive items carry `data-destructive`. Alpine handles
  open/close, click-outside-closes (`@click.outside`), arrow-key /
  Home / End navigation, Escape closes + returns focus to trigger.
- **Context**: `items: Vec<{id, label, deleted_at, deleted_by,
  reason}>`, `pagination`. Scaffold seeds 4 tombstones (including
  one nullable `deleted_by` / `reason` to exercise the
  `default(value="…")` filters and one GDPR Article 17 erasure row).
- **Real wiring**: `Repository::list_soft_deleted(page, limit)` +
  the planned `POST /api/{plural}/{id}/restore` + `DELETE
  /api/{plural}/{id}/purge` REST endpoints. The index page's top
  `action-bar` carries a "Trash" link to this page.

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
  action-bar; top `action-bar` carries "API tokens" link to `/tokens`.

### `tokens.html.tera` — `GET /tokens`

- **Components**: `breadcrumb-nav` (Home / Settings / API tokens),
  guidance `hint` paragraph, top `action-bar` ("+ Generate new token"
  + back to Settings), one-time-reveal `alert` (`data-type="warning"`,
  shown only when `justCreated` is non-null) containing a `secret-input`
  with `data-state="revealed"` and `clipboard-copy-button` plus a
  Dismiss button (clears `justCreated` permanently), `data-table`
  with Label + `tag-group` of scope tags (`admin` scope renders
  `data-type="warning"`) + masked **`secret-input`** with reveal/hide
  toggle + clipboard-copy + Created + Last used (or "never") + Expires
  (or "never") + Revoke `action-bar-button` gated by per-row
  `alert-dialog`, generate-token `drawer` with label `text-input` +
  `fieldset` of scope `checkbox-input`s (template-iterated from
  `availableScopes`) + expiry `select`, `drawer-backdrop`.
- **`secret-input` contract**: inline-flex container with
  `aria-label`, `data-state="masked"` / `"revealed"`; child
  `secret-input-value` `<code>` carries the token text (`user-select:
  all` for triple-click); child `secret-input-toggle` `<button>` is
  the Reveal / Hide button; Reveal arms a 10-second Alpine
  `setInterval` countdown that auto-hides; Hide button's
  `aria-label` interpolates the remaining seconds for screen readers;
  always-available `clipboard-copy-button` sibling.
- **Context**: `tokens: Vec<{id, label, scopes: Vec<String>,
  preview_full, last4, created_at, last_used_at?, expires_at?}>`.
  Scaffold seeds 4 tokens including one with null `last_used_at`
  (renders "never"), one with null `expires_at` (admin break-glass,
  renders "never"), and one with `admin` scope to exercise the
  warning-styled tag.
- **Real wiring**: `TokenRepository::list_for_user(user_id)` +
  `POST /api/tokens` (returns the secret exactly once in the response
  body) + `DELETE /api/tokens/{id}` (revoke). Token generation is
  intentionally client-side in the scaffold so the secret never
  round-trips through server response logs; production swaps the
  Alpine `generate()` for an `hx-post`.

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
