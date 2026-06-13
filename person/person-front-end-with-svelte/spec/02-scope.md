## 2. Scope

### 2.1 In scope (MVP)

- Dashboard with service-health + recent audit feed.
- Persons list with full-text / fuzzy / phonetic search and SVAR DataGrid.
- Create person with 409-duplicate handling that surfaces the match candidates inline.
- Detail view: identity, identifiers, addresses, telecom, emergency contacts.
- Edit form (full Person record).
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

