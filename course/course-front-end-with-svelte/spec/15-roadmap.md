## 15. Roadmap

- **v0.2** (shipped 2026-06-05): bug-fix sweep against the now-real
  Course Service surface — `ScoredCandidate` alignment, blank-string
  URL normalisation, inert match-page threshold, dashboard health
  badge, empty-search `"*"` wildcard, inert phonetic checkbox,
  `API_BASE_URL` default + README/`.env.example` port realignment.
  Plus spec.md §13 / §14 counter realignment + CHANGELOG.
- **v0.3** (next): SSR-safe load functions using `event.fetch` for
  warm-cache SEO-irrelevant wins (T-13); Lily Dialog (merge
  confirm) + Combobox (identifier system) integration (T-14);
  instance / syllabus-section edit UI (T-15 — the genuine
  remaining sub-resource gap).
- **v0.4** (shipped 2026-06-18, documented retroactively this DOC-4
  pass): BFF auth (T-24) — httpOnly-cookie + PASETO model per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
  (no client-held bearer / `localStorage`); i18n (13 locales) + Lily
  `ThemePicker`/`LocalePicker` (T-25); landed in the same commit as
  the family-wide auth migration but had gone unrecorded in this
  crate's own spec/CHANGELOG until now. CSRF + route-level auth
  guards remain open (T-26).
- **v0.4.1** (shipped 2026-07-19): `/board` (SVAR Kanban) and
  `/calendar` (SVAR Calendar) routes.
- **v0.5+**: Batch dedup results UI (T-18), masked-view toggle on
  detail (T-19), GDPR-export download (T-20), CSRF + route guard
  (T-26), `/signin`/`/verify` i18n (T-27).

