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
- Theme switcher in the layout shell (Lily `ThemePicker`, persisted to
  `localStorage` under `lily-theme`; DaisyUI themes plus the bespoke NHS
  England / Scotland / Wales patient & practitioner themes).
- Locale switcher in the layout shell (Lily `LocalePicker`, persisted to
  `localStorage` under `mxi.person.locale` — owned by the app's own
  `i18n.svelte.ts` store, not a `LocalePicker`-internal key, with
  navigator detection), backed
  by a full translated string catalogue for all 13 supported locales
  (parity-tested — see `agents/share/locales.md` and
  `tests/unit/i18n.test.ts`).
- Cross-service links panel on the detail page (assert/list/withdraw
  `same_identity`/`works_at`/`member_of` edges — §6 FR-21, §13 T-23).
- Bulk import/export screen at `/persons/bulk` (§6 FR-22, §13 T-24).
- Duplicate review-queue screen at `/review` (§6 FR-14…FR-20, §13 T-25).
- Sign-in / sign-out via the BFF magic-link flow (`/signin`, `/verify`;
  §13 T-22) — httpOnly session cookie + server-side PASETO exchange, no
  token in browser JS.

### 2.2 Out of scope (MVP)

- FHIR R5 resource viewer.
- GDPR data-export download (the API exists; no UI yet).
- Consent management UI.
- Identity-document detail editing (read-only on detail page).
- Batch deduplication scan UI (API exists; defer until ops asks).
- CSRF synchroniser token on mutating browser→BFF calls (§13 T-22 remaining work; `SameSite=Lax` is the only backstop today).

