# Module 6 — Content insights

## Principle

Every number is **derived by pure-core arithmetic from recorded
facts** — the family posture (patient-flow capacity, PPM burndown,
CRM dashboards). No insight is a stored, editable field; all are
ETag-conditional reads stamped `as_of` and carrying the **rule that
produced each finding**, so an editor can argue with the rule
rather than guess at it.

These are **editorial** insights. There is no reader analytics here
— CMS records no visits and holds no visitor identity
([delivery](delivery.md), [scope](scope.md)).

## Content health

| Finding | Rule |
|---|---|
| **Missing alt text** | a published variant references an image asset whose `alt_text` is empty ([assets](assets.md)) |
| **Missing SEO** | published + `index`-able, with no `meta_title` or no `meta_description` |
| **Broken reference** | a reference whose target is missing, archived, or (for a published referrer) unpublished |
| **Orphan asset** | an asset with no reference from any non-archived revision |
| **Stale content** | published with no revision for N days (per content type, default 365) |
| **Stale translation** | source published past the translated revision — with the count of source revisions behind ([localization](localization.md)) |
| **Stuck in review** | `in_review` for longer than the site's review SLA (default 7 days), with the assigned reviewer |
| **Unscheduled approval** | `approved` but neither published nor scheduled for longer than N days |
| **Needs migration** | a revision written under an older `type_schema_version` that a later tightening would now reject ([authoring](authoring.md)) |
| **Route hazard** | redirect chains near the hop cap, `noindex` pages linked from a menu, duplicate canonical targets |

Each finding carries the entry, variant, locale, rule key, the
observed values, and the actor best placed to fix it (owner or
reviewer). No severity score is invented: findings are grouped by
rule, and the count is the count.

## Editorial throughput

| View | Derivation |
|---|---|
| **Activity by state** | submissions / approvals / rejections / publishes / unpublishes per period, from the audit and event record |
| **Time in state** | median and p90 draft→review, review→approved, approved→published, per content type and period |
| **Per-actor** | authored revisions, reviews completed, publishes — scoped by the persona rules ([auth](auth.md)) |
| **Publishing cadence** | published variants per period per site and locale |
| **Locale coverage** | per site: entries with a published variant per locale, and the gap list |
| **Backlog** | open translation requests, pending reviews, pending schedules, each with age buckets |

## Honesty rules

- Every ratio reports numerator and denominator; a zero denominator
  renders `null` plus the absolutes, never `0%` or `100%`.
- Percentiles state the sample size; below a floor (default 5) the
  view returns the raw durations instead of a percentile that means
  nothing.
- "Time in state" is measured from recorded transition events, not
  from `updated_at` — a field that moves for unrelated reasons.
- Views carry `as_of` and the filter that produced them.
- Per-actor views are **for coaching and audit, and are scoped by
  policy** — an author persona sees their own numbers, not a
  leaderboard of colleagues.
