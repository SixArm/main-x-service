## 5. Information Architecture

| Route | Purpose |
| --- | --- |
| `/` | Dashboard |
| `/courses` | List + search |
| `/courses/new` | Create |
| `/courses/match` | Match check |
| `/courses/merge` | Merge |
| `/courses/[id]` | Detail |
| `/courses/[id]/edit` | Edit |
| `/courses/[id]/audit` | Audit log |

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.
