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
- **v0.4**: Auth integration once Course Service ships JWT (T-15 of
  service spec — blocked on the family-wide rollout).
- **v0.5+**: Batch dedup results UI (T-18), masked-view toggle on
  detail (T-19), GDPR-export download (T-20).

