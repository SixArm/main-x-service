## 5. Information Architecture

| Route | Purpose |
| --- | --- |
| `/` | Dashboard |
| `/places` | List + search |
| `/places/new` | Create |
| `/places/match` | Match check |
| `/places/merge` | Merge |
| `/places/[id]` | Detail |
| `/places/[id]/edit` | Edit |
| `/places/[id]/audit` | Audit log |
| `/review` | Duplicate-review board |
| `/signin` | Magic-link sign-in (BFF) |
| `/verify` | Magic-link verification (BFF) |

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.
