# Accessibility checklist

> Part of the [Svelte edition specification](index.md). The UI is the
> surface a clinician touches; the project targets **WCAG 2.2 AA** (see
> [regulatory.md](regulatory.md)).

- [x] Single `<h1>` per page (in `+layout.svelte`'s header).
- [x] Logical heading order (`h1` → `h2` → `h3`).
- [x] Skip link as first focusable element.
- [x] Landmark roles (`header`, `nav`, `main`, `footer`).
- [x] Form fields paired with visible labels via `Field`.
- [x] Live regions for success / error feedback.
- [ ] Tested with VoiceOver, NVDA, JAWS (manual — TODO).
- [ ] Automated axe scans in CI (TODO).
