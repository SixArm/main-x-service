# AGENTS.md — Organization Front-End

Operator UI for the [Organization Service](../organization-service-rust-crate):
organization CRUD + matching.

> Read [`spec/index.md`](./spec/index.md) first — the living spec.

## What this is

A SvelteKit 2 / Svelte 5 (runes) **SPA**. It calls the organization
service REST API, whose request/response body is the
`organization_matcher::Organization` shape itself.

## Ground rules

1. **Runes only** (`$state`/`$derived`/`$effect`/`$props`/`$bindable`).
   No `export let`, no `$:`, events are callback props.
2. **SPA.** `+layout.ts` sets `ssr = false` / `prerender = false`.
3. **TypeScript strict** (`noUncheckedIndexedAccess`).
4. **Minimal deps.** No data grid / design system — plain inputs + the
   `app.css` utilities. (Drift from the SVAR/Lily front-ends is accepted
   family-wide.)
5. **No envelope.** The service is loco.rs and returns **raw JSON**;
   `src/lib/api/client.ts` is the lean wrapper (get/post/put/delete).

## Layout

```
src/
├── lib/
│   ├── config.ts                 PUBLIC_API_BASE_URL (default :5150)
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (+ ApiError)
│   │   ├── types.ts              Organization + OrgIdentifier + PostalAddress + OrgRef + ScoredRef
│   │   └── organizations.ts      OrganizationRepository (CRUD + checkDuplicates)
│   └── components/OrganizationForm.svelte
└── routes/
    ├── +layout.svelte / +layout.ts   nav + SPA toggle
    ├── +page.svelte              list
    ├── new/+page.svelte          create
    ├── [pid]/+page.svelte        detail + delete + check-duplicates
    └── [pid]/edit/+page.svelte   edit
```

## API consumption

| UI action | Endpoint |
|---|---|
| List | `GET /api/organizations` |
| Create | `POST /api/organizations` |
| Detail | `GET /api/organizations/{pid}` |
| Edit | `PUT /api/organizations/{pid}` |
| Delete | `DELETE /api/organizations/{pid}` |
| Check duplicates | `POST /api/organizations/check-duplicates` |

## Commands

```bash
pnpm install
pnpm dev          # http://localhost:5173
pnpm run check    # svelte-check (strict; 0/0 expected)
pnpm run build
```

Configure the API base URL with `PUBLIC_API_BASE_URL` (see `.env.example`).
