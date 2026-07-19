## 5. Information Architecture

| Route | Purpose |
| --- | --- |
| `/` | Dashboard |
| `/events` | List + search |
| `/events/new` | Create |
| `/events/match` | Match check |
| `/events/merge` | Merge |
| `/events/[id]` | Detail |
| `/events/[id]/edit` | Edit |
| `/events/[id]/audit` | Audit log |
| `/calendar` | SVAR Calendar over the event time-window; drag-to-reschedule |
| `/signin` | Magic-link sign-in (BFF) |
| `/verify` | Magic-link verification |

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.
