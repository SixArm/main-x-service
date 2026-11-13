## 16. Open Questions

Open questions resolve into §13 tasks or §5–§12 amendments when
decisions are made. Matcher-internal questions live in the matcher
spec §16.

- ~~**OQ-1 — Validation status code.**~~ **Resolved 2026-06-13
  (T-2): `422` is normative**, matching the family convention
  (person/place services) and the service crate spec §6. axa
  has no `unprocessable_entity` helper, so the controller returns
  `Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)` from a
  shared `validate()` used by create and update. `400` remains for
  malformed bodies (axa JSON rejection). See §9.1 and §13 T-2.
- **OQ-2 — Duplicate-check scale strategy.** `check-duplicates`
  scans at most 1 000 stored rows in memory. At national volumes:
  search-based blocking (Tantivy), JSONB GIN pre-filtering on
  condition codes, or both? (Feeds T-6.)
- **OQ-3 — Normalising condition codes.** Keep the pure-JSONB model,
  or break `condition_codes` / `interventions` into side tables once
  search and code-level queries land? (Crate spec carries the same
  question.)
- **OQ-4 — Real-time duplicate detection on create.** Mature
  siblings return `409 Conflict` with candidates on create; today
  duplicate checking is explicit-only. Adopt the `409` behaviour, or
  keep create cheap and rely on the front-end calling
  `check-duplicates` first?
- **OQ-5 — Cross-language duplicates.** The same NICE-style pathway
  translated into another language scores low on name similarity.
  Rely solely on deterministic identifiers / `same_as`, or add a
  translation-aware name component (roadmap localization work)?
- **OQ-6 — Provider identity.** `provider_id` is a free string.
  Should it reference the
  [organization entity](../../organization/) (`pid`) so provider
  scoping survives organisation renames and merges?
- **OQ-7 — Fairness slices need demographics the instance layer does
  not hold.** ehrapy's `detect_bias` (standardised mean differences and
  value-count ratios across sensitive attributes) and Cox case-mix
  adjustment would answer "who is breaching the standard", which is the
  question an equity audit asks. The instance layer carries only a
  `subject_ref` URN, by design — age, sex, ethnicity, and deprivation
  live in the [person](../../person/) registry. Options: (a) a
  server-side join through the person service under the caller's
  forwarded credential, aggregated and suppressed before it leaves; (b)
  the caller exports `journey_features` (T-14a) and joins in a secure
  processing environment; (c) do not offer it. (a) makes this service a
  processor of demographic data it never stored, with everything that
  implies for §12; (b) keeps the join where the data governance already
  is. *Lean: (b), and say so in the T-14a docs — a fairness slice is
  exactly what the feature export is for.* (Raised by the T-14 triage,
  2026-09-03.)
- **OQ-8 — Cost on the process map.** BNSSG annotated its
  directly-follows map with median cost per activity, which turned a
  flow diagram into a spend diagram. No `CarePathway`, instance, or
  segment field carries cost, and adding one is a domain expansion (whose
  cost? reference cost, tariff, actual?) rather than an analytics
  feature. *Lean: leave the map cost-free; if a cost field ever lands on
  segments, T-14b's node annotation is the one-line change.*
- **OQ-9 — What is a "case" in the event log?** T-14a uses the instance
  `pid` as `case_id`, so one patient with a re-enrolment or a
  `continues_as` continuation appears as two cases — BNSSG had the same
  problem the other way round (case = patient, so a second hip
  replacement was truncated). The stitched journey
  (`GET /api/instances/{pid}/journey`) is the natural case for a
  cross-episode analysis, but it crosses a service boundary and a
  credential boundary. *Lean: instance now; add an optional
  `journey_id` column populated only when the caller can read every
  leg, once T-14a exists.*
- **OQ-10 — Families as people the pathway serves (PICS-F).** The ICU
  psychology pathway triaged in T-15 supports relatives as well as
  survivors. Post-intensive-care syndrome in family members is a
  recognised condition, and families co-produced the pathway. An
  instance has one `subject_ref`. Options: (a) a relative gets their
  own instance on a family-support template, unlinked; (b) the same,
  plus a link between the two instances; (c) family contacts are
  events on the patient's instance, naming no one. (b) is a new
  patient-to-patient relationship, which is more sensitive than any v1
  edge kind and is not in the closed `entity-ref` registry. *Lean: (a)
  and (c) now; (b) only with its own §10-style governance decision.*
  (Raised by the T-15 triage, 2026-11-13.)
- **OQ-11 — Eligibility for a standard the instance cannot see.** NICE
  QS158 s4 applies to adults with **more than 4 days in critical care**
  who are at risk of morbidity. The instance layer does not know
  critical-care length of stay. A `patient_flow_stay` leg does, once
  patient-flow serves a timeline. Options: (a) the caller filters the
  cohort and the response says eligibility was not checked; (b) the
  standard reads LOS from the stitched journey under the caller's
  credential; (c) an operator-declared eligibility flag on the
  instance. *Lean: (a) now, as T-15f states; (b) once the
  patient-flow timeline exists, because (c) is an assertion nobody
  verifies.*
- **OQ-12 — Do reflective review cycles belong in a registry at all?**
  T-15g would store PDSA / ROOTS reviews beside the template they
  changed. Against: ROOTS is about a team, not a pathway identity; its
  value is the facilitated conversation; a stored RAG is one query away
  from the league table the manual refuses. For: the link between "we
  changed the pathway" and "here is what the next cohort looked like"
  (T-15d) is otherwise kept nowhere. *Lean: build only on a
  deployment's request, with T-15g's refusals as acceptance criteria,
  and revisit whether `organization` should own the team-level half.*
