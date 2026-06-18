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

### 2.2 Out of scope (MVP)

- Authentication / authorisation UI.
- FHIR R5 resource viewer.
- GDPR data-export download (the API exists; no UI yet).
- Consent management UI.
- Identity-document detail editing (read-only on detail page).
- Batch deduplication scan UI (API exists; defer until ops asks).
- i18n / locale switching.
- Theme switcher.

