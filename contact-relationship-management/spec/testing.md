# Testing

- **Pure-core unit tests** (DB-free, exhaustive): every lifecycle's
  legal/illegal transition matrix (lead, deal incl. terminal
  immutability + reasoned reopen, campaign, ticket incl. reopen,
  article); the scoring rule table + clamp + breakdown sums;
  forecast arithmetic (per-currency separation, empty pipelines);
  ROI incl. zero-cost `null`; CLV; SLA deadline derivation +
  breach flip + priority-change re-derivation; segment filter
  evaluation incl. the consent AND-gate; overflow refusal on all
  money sums.
- **Request tests** (Postgres, `#[ignore]`d): the sales journey
  (capture lead → score → convert → deal through stages → won →
  forecast reflects it); the marketing journey (consent → segment
  preview → campaign run (simulated) → attribution → unsubscribe
  exits nurture and blocks the next send); the support journey
  (open → first response stamps → resolve; priority change
  re-derives; breach sweep emits once); Kanban reorder race
  (`FOR UPDATE`, one winner); nurture scheduler idempotency (rerun
  ⇒ no double-send); unknown-pid 404s (family lesson, pinned from
  day one).
- **Enforcement binary** (own process — the OnceLock lesson): the
  persona matrix (rep `$sub` ownership vs other-rep refusal,
  manager team scope, marketing pipeline read-only, support scope,
  `mask` obligation on amounts/forecast).
- **Front-end**: vitest for the API client path map, `money()`,
  score-breakdown + SLA-countdown components; Playwright over a
  `page.route`-stubbed API (contract-mirroring, unstubbed =
  404-loud).
- Seed task: a synthetic book of business (~50 contacts, ~15
  accounts, leads in every status, 2 pipelines, ~30 deals, a
  campaign + nurture sequence, ~20 tickets against 2 SLA policies)
  — synthetic data only.
