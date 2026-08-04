## 2. Scope

### 2.1 In scope (MVP)

- Dashboard with service-health + recent audit feed.
- Events list with full-text / fuzzy search (plus date-range and status / type filters) and SVAR DataGrid. (Phonetic search is not a service search parameter; see §06 FR-2.)
- Create event with 409-duplicate handling that surfaces the match candidates inline.
- Detail view: identity (time window, status, type, attendance mode, time zone, duration), locations, organizers, performers, identifiers, offers.
- Edit form (full Event record).
- Soft-delete (with confirm).
- Match check page (score a hypothetical record against the index).
- Merge UI (preview + execute).
- Per-record audit log view.
- SVAR Calendar view (`/calendar`) with drag-to-reschedule.
- Authentication (BFF): magic-link sign-in (`/signin`, `/verify`), httpOnly session cookie, server-side PASETO exchange, `/api/proxy` reverse proxy — see §13 T-23. CSRF on mutating browser→BFF calls is **not yet done** (§16 OQ-3).
- i18n / locale switching (13 locales via Lily `LocalePicker`; the `/signin` and `/verify` pages are not yet translated — plain English only, a known follow-up).
- Theme switcher (Lily `ThemePicker`).

### 2.2 Out of scope (MVP)

- FHIR R5 resource viewer.
- GDPR data-export download (the API exists; no UI yet).
- Consent management UI.
- Identity-document detail editing (read-only on detail page).
- Batch deduplication scan UI (API exists; defer until ops asks).

