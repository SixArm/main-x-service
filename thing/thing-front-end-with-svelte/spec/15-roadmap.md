## 15. Roadmap

- **v0.2**: SSR-safe load functions; Lily Dialog/Combobox integration; remaining-field edit UI (T-15: images, main_entity_of_page, subject_of, potential_action).
- ~~**v0.3**: Auth integration (once Thing Service ships auth).~~ **Done, 2026-07-04** (`f66ff50f`): the family-wide auth-service migration landed independently of any per-service auth work — BFF + httpOnly-cookie + PASETO model per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md), no client-held bearer / `localStorage` (§8, §13 T-22). Remaining under that umbrella: CSRF (T-22) and E2E coverage for the auth pages (T-23).
- **v0.4**: Sibling scaffolds for the other entity front-ends (copy-adapt; accept drift per project decision 2026-06-02 — siblings now exist for every entity). Done as a milestone; drift is the accepted steady state, not a remaining task.
- **v0.5 (new)**: `check-duplicates` preview wired into the create form (T-17); masked-view toggle (T-19); GDPR-export download (T-20); SVAR Kanban/Calendar/Gantt/FileManager seams beyond `/review` are installed but unrouted (`CHANGELOG.md` 2026-07-19) pending a data-gated feature (e.g. warranty/maintenance dates — the Thing model carries none today).

