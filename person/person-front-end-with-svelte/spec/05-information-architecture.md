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
| `/review` | Duplicate-review board — SVAR Kanban (drag-to-decide) + a keyboard-reachable queue table, with an inline comparison sub-state (see below) |
| `/expiry` | Identity-document expiry calendar (SVAR Calendar) |
| `/signin` | Magic-link sign-in (BFF) |
| `/verify` | Magic-link verification (BFF) |

### `/review` sub-states

The route has no child routes; selecting a pair is an **in-page state**,
not a URL, because the queue is not addressable server-side (the list
endpoint takes no id and there is no `GET /review-queue/{id}`, so a
deep-linked selection could not be restored on a cold load).

| Sub-state | Trigger | Effect |
| --- | --- | --- |
| Board + queue (default) | route load | Kanban columns over the four wire statuses, plus the queue table |
| Comparison open | clicking a card, or a queue row's `Compare` button | inline panel below the board: both records side by side, the score breakdown, and the decide buttons; focus moves to the panel |
| Decided | `Confirm` / `Reject` in the panel, or a drag on the board | the decision endpoint is called, the list reloads through the current filter, and the panel stays open on the decided item so its merge deep-link is reachable |

The one navigation this route *does* emit is the merge deep link:
`/persons/merge?main=…&duplicate=…`, in either survivor order.

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.
