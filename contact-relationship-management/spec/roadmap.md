# Roadmap

Beyond the v1 queue ([tasks.md](tasks.md)):

- **Real ESP adapter** — the campaign/nurture send seam gains an
  actual email-service-provider implementation (bounces,
  deliverability, webhooks) behind the existing trait.
- **Event-driven merge repointing** — consume upstream `merged`
  events on the durable bus to repoint contact/account wrappers
  automatically (manual endpoint in v1).
- **Multi-touch attribution** — first/last/linear models beyond v1
  source attribution.
- **Business-hours SLA clocks** — pause-on-pending, holiday
  calendars.
- **Public KB portal** + ticket deflection metrics.
- **Quotas & territories; commission views.**
- **CLV refinement** — margin, discounting, churn-adjusted models
  over the v1 revenue sum.
- **Cross-app bridges** — WPM (rep identity/teams from employee
  records), PPM (post-sale delivery projects opened from won
  deals).
- **FX normalization** for mixed-currency roll-ups (rate source +
  as-of discipline).
