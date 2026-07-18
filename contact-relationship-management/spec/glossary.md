# Glossary

| Term | Meaning |
|---|---|
| **Account** | The relationship wrapper over an `organization:` record |
| **Attribution** | Crediting a lead/deal's revenue to a campaign (`source_campaign_pid`; v1 = source attribution) |
| **CLV** | Customer Lifetime Value; v1: Σ won-deal amounts per account, per currency |
| **Consent** | Recorded permission to market to a contact; append-only history; send-path enforced |
| **Contact** | The relationship wrapper over a `person:` record |
| **Deal** | A revenue opportunity moving through pipeline stages (a.k.a. opportunity) |
| **Drip / nurture** | An ordered, delayed sequence of marketing touches per enrolled contact |
| **Forecast** | Stage-weighted Σ (amount × probability) over open deals — derived, never typed |
| **Kanban** | The board view of deals grouped by stage with manual ordering |
| **Lead** | An unqualified prospect; scored by deterministic rules; converts to contact (+ deal) |
| **MQL / SQL** | Marketing-/Sales-Qualified Lead; in v1 the `warm`/`hot` score labels |
| **Pipeline** | A named, ordered list of stages with win probabilities and terminal flags |
| **ROI** | (attributed won revenue − cost) ÷ cost, per campaign |
| **SFA** | Sales Force Automation — module 1 |
| **SLA** | Service Level Agreement: per-priority first-response/resolution targets with derived deadlines |
| **Segment** | A saved, declarative audience filter over contacts (consent always ANDed) |
| **Stalled deal** | An open deal with no stage move or activity for N days (default 14) |
| **Ticket** | An operational support record — not a `case`-registry identity |
| **Win rate** | won ÷ (won + lost) over closed deals |
