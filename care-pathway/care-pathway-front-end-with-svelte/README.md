# care-pathway-front-end-with-svelte

Operator UI for the [Care Pathway Service](../care-pathway-service-with-loco):
care-pathway **CRUD + matching + name search + merge + audit trail +
recent activity**.

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA.

## Routes

| Route | Purpose |
|---|---|
| `/` | List care pathways + name-search box + recent-activity toggle |
| `/new` | Create |
| `/[pid]` | Detail + delete + check-duplicates + merge + audit-trail toggle |
| `/[pid]/edit` | Edit |

Auth (target model): **Sign in** via the central authentication-service
magic-link establishes a server-side **cookie session**
(`__Host-mxi_session`, httpOnly); the browser holds **no token** and
talks only to this front-end's own SvelteKit server (BFF), which
exchanges the session for a short-lived **PASETO v4.public** token and
calls the care pathway service server-side. Mutating requests are
CSRF-protected; there is no `localStorage` and no `mxi_access_token`.
Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256 JWT + JWKS and the `#access_token` fragment handoff
decommissioned). **Pivot in progress** — the current runtime still uses
the older client-held-token flow; code follow-up tracked in spec §13.

## Prerequisites

- Node 20+ and pnpm
- A running [Care Pathway Service](../care-pathway-service-with-loco)

## Quick start

```bash
cp .env.example .env     # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                 # http://localhost:5173
```

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `PUBLIC_API_BASE_URL` | `http://localhost:5150` | Care pathway service REST base URL. |
| `VITE_AUTH_FRONTEND_URL` | `http://localhost:5173` | Central authentication front-end base URL for sign-in. Target model is a server-side cookie session + BFF (no browser token); see [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md). Pivot in progress — code follow-up tracked in spec §13. |

## How it works

The care-pathway record body **is** the `care_pathway_matcher::CarePathway`
shape (name, pathway code, provider, care setting, target condition codes
(ICD/SNOMED), interventions, keywords, identifiers, sameAs). The form
edits these; `check-duplicates` posts the current record and lists stored
matches with their scores. The list page offers a name-search box
(`GET /search?q=`) and a recent-activity toggle (`GET /events/recent`);
the detail page offers a per-row **Merge into this record** action
(`POST /merge`) and a per-pathway audit-trail toggle (`GET /{pid}/audit`).

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm run build
pnpm test          # vitest unit suite
pnpm test:e2e      # Playwright smoke (runs against `vite preview`)
```

## License

Dual-licensed under MIT OR Apache-2.0.
