# care-pathway-front-end-with-svelte

Operator UI for the [Care Pathway Service](../care-pathway-service-rust-crate):
care-pathway **CRUD + matching**.

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA.

## Routes

| Route | Purpose |
|---|---|
| `/` | List care pathways |
| `/new` | Create |
| `/[pid]` | Detail + delete + check-duplicates |
| `/[pid]/edit` | Edit |

## Prerequisites

- Node 20+ and pnpm
- A running [Care Pathway Service](../care-pathway-service-rust-crate)

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
| `VITE_AUTH_FRONTEND_URL` | `http://localhost:5173` | Central authentication front-end base URL. "Sign in" redirects to `${VITE_AUTH_FRONTEND_URL}/signin?return_to=…`; the access token is handed back via the URL fragment (cross-origin SSO; see `agents/share/jwt-enforcement.md`). |

## How it works

The care-pathway record body **is** the `care_pathway_matcher::CarePathway`
shape (name, pathway code, provider, care setting, target condition codes
(ICD/SNOMED), interventions, keywords, identifiers, sameAs). The form
edits these; `check-duplicates` posts the current record and lists stored
matches with their scores.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm run build
```

## License

Dual-licensed under MIT OR Apache-2.0.
