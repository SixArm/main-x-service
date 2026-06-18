## 2. Scope

### 2.1 In scope (MVP)

- Dashboard with service-health + recent audit feed.
- Courses list with full-text / fuzzy search and SVAR DataGrid. (Phonetic search is out of scope until the service grows a real Soundex search path — see §2.2 and FR-2.)
- Create course with 409-duplicate handling that surfaces the match candidates inline.
- Detail view: identity (course code, status, educational level, credits, time required), identifiers, teaches, keywords, alternate names, same-as links, instances (read-only).
- Edit form (full Course record).
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
- Phonetic (Soundex) list search (the service param is a documented no-op; defer until a real Soundex search path ships — §13 T-22).
- i18n / locale switching.
- Theme switcher.

