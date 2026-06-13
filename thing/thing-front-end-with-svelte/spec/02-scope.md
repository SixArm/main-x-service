## 2. Scope

### 2.1 In scope (MVP)

- Dashboard with service-health + recent audit feed.
- Things list with full-text / fuzzy / phonetic search and SVAR DataGrid.
- Create thing with 409-duplicate handling that surfaces the match candidates inline.
- Detail view: identity, identifiers, alternate names, same-as URLs, images.
- Edit form (full Thing record).
- Soft-delete (with confirm).
- Match check page (score a hypothetical record against the index).
- Merge UI (preview + execute).
- Per-record audit log view.

### 2.2 Out of scope (MVP)

- Authentication / authorisation UI.
- FHIR R5 resource viewer.
- GDPR data-export download (the API exists; no UI yet).
- Consent management UI.
- Image-list editing (images render on the detail page; no edit UI yet).
- Batch deduplication scan UI (API exists; defer until ops asks).
- i18n / locale switching.
- Theme switcher.

