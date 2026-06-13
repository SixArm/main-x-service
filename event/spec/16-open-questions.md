## 16. Open Questions

Entity-level questions (prefix `EOQ-`); per-crate questions stay in
the owner's §16 / §10.

- **EOQ-1 — One matching algorithm or two?** The in-service matcher
  powers the REST endpoints; the embedded matcher crate is the
  canonical reference. Their weight tables and components differ
  (§6.1): the in-service matcher scores attendees and window
  overlap; the crate scores category, country code, and URL, and
  renormalises over missing fields. Options: (a) route REST scoring
  through the crate via the adapter, (b) upstream the in-service
  components into the crate then do (a), (c) keep both and document
  the split permanently. Tracked as ET-4.
- **EOQ-2 — Recurrence ownership.** When RRULE lands (roadmap item
  5): does expansion live in the service only, or does the matcher
  need recurrence-aware comparison (two records describing the same
  weekly series vs two occurrences)? Are occurrences materialised as
  `sub_events` under a series Event, or virtual?
- **EOQ-3 — SVAR DataGrid licensing for governmental deployment.**
  `wx-svelte-grid` free tier is GPL-3.0. A public-sector deployment
  may be fine with GPL or may require the Pro/Enterprise licence —
  legal review needed before production. (Front-end §16 OQ-1 raises
  this for the commercial case; the governmental case is decided
  here.)
- **EOQ-4 — Attendee data minimisation at population scale.** Should
  the registry hold per-Event attendee lists at all, given Art. 9
  inference risk (§12.1)? Alternatives: hold only counts +
  organizer/performer parties, or hold attendee references with
  field-level encryption. Interacts with the GDPR Art. 17
  erasure-vs-audit-retention tension (audit rows snapshot old/new
  JSON including party data).
- **EOQ-5 — Match-quality vocabulary.** The service classifies
  Definite / Probable / Possible / Unlikely; the matcher reports
  High / Medium / Low confidence. Operators see the former; bridge
  consumers see the latter. Unify, or document the mapping in the
  adapter?
