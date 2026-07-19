## 5. Information Architecture

| Route | Purpose |
| --- | --- |
| `/` | Dashboard |
| `/persons` | List + search |
| `/persons/new` | Create |
| `/persons/match` | Match check |
| `/persons/merge` | Merge |
| `/persons/[id]` | Detail |
| `/persons/[id]/edit` | Edit |
| `/persons/[id]/audit` | Audit log |
| `/review` | Duplicate-review board (SVAR Kanban, drag-to-decide) |
| `/expiry` | Identity-document expiry calendar (SVAR Calendar) |
| `/signin` | Magic-link sign-in (BFF) |
| `/verify` | Magic-link verification (BFF) |

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.
