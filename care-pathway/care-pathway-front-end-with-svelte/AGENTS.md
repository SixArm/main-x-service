# AGENTS.md — Care Pathway Front-End

Operator UI for the [Care Pathway Service](../care-pathway-service-rust-crate):
care-pathway CRUD + matching.

> Read [`spec/index.md`](./spec/index.md) first — the living spec.

## What this is

A SvelteKit 2 / Svelte 5 (runes) **SPA**. It calls the care-pathway
service REST API, whose request/response body is the
`care_pathway_matcher::CarePathway` shape itself.

## Ground rules

1. **Runes only** (`$state`/`$derived`/`$effect`/`$props`/`$bindable`).
   No `export let`, no `$:`, events are callback props.
2. **SPA.** `+layout.ts` sets `ssr = false` / `prerender = false`.
3. **TypeScript strict** (`noUncheckedIndexedAccess`).
4. **Minimal deps.** No data grid / design system — plain inputs + the
   `app.css` utilities.
5. **No envelope.** The service is loco.rs and returns **raw JSON**;
   `src/lib/api/client.ts` is the lean wrapper (get/post/put/delete).

## Layout

```
src/
├── lib/
│   ├── config.ts                 PUBLIC_API_BASE_URL (default :5150)
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (+ ApiError)
│   │   ├── types.ts              CarePathway + ConditionCode + CareSetting + IdentifierScheme + PathwayRef + ScoredRef
│   │   └── care-pathways.ts      CarePathwayRepository (CRUD + checkDuplicates + merge)
│   └── components/CarePathwayForm.svelte
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
| List | `GET /api/care-pathways` |
| Create | `POST /api/care-pathways` |
| Detail | `GET /api/care-pathways/{pid}` |
| Edit | `PUT /api/care-pathways/{pid}` |
| Delete | `DELETE /api/care-pathways/{pid}` |
| Check duplicates | `POST /api/care-pathways/check-duplicates` |
| Merge duplicate | `POST /api/care-pathways/merge` |

## Commands

```bash
pnpm install
pnpm dev          # http://localhost:5173
pnpm run check    # svelte-check (strict; 0/0 expected)
pnpm run build
```

Configure the API base URL with `PUBLIC_API_BASE_URL` (see `.env.example`).
