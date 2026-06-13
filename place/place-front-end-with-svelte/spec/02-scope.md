## 2. Scope

### 2.1 In scope (MVP)

- Dashboard with service-health + recent audit feed.
- Places list with full-text / fuzzy / phonetic search and SVAR DataGrid.
- Create place with 409-duplicate handling that surfaces the match candidates inline.
- Detail view: identity, address, geo coordinates, identifiers, opening hours, amenities.
- Edit form (full Place record).
- Soft-delete (with confirm).
- Match check page (score a hypothetical record against the index).
- Merge UI (preview + execute).
- Per-record audit log view.

### 2.2 Out of scope (MVP)

- Authentication / authorisation UI.
- FHIR R5 resource viewer.
- GDPR data-export download (the API exists; no UI yet).
- Consent management UI.
- Sub-record list editing (identifiers, opening hours, amenities are read-only on the detail page — see §13 T-15).
- Batch deduplication scan UI (API exists; defer until ops asks).
- i18n / locale switching.
- Theme switcher.

