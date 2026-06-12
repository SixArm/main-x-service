# organization-front-end-with-svelte

Operator UI for the [Organization Service](../organization-service-rust-crate):
organization **CRUD + matching**.

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA.

## Routes

| Route | Purpose |
|---|---|
| `/` | List organizations |
| `/new` | Create |
| `/[pid]` | Detail + delete + check-duplicates |
| `/[pid]/edit` | Edit |

## Prerequisites

- Node 20+ and pnpm
- A running [Organization Service](../organization-service-rust-crate)

## Quick start

```bash
cp .env.example .env     # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                 # http://localhost:5173
```

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `PUBLIC_API_BASE_URL` | `http://localhost:5150` | Organization service REST base URL. |

## How it works

The organization record body **is** the `organization_matcher::Organization`
shape (name, legalName, alternateName, identifiers (LEI/DUNS/…), url,
sameAs, address, jurisdiction, foundingDate, keywords). The form edits
these; `check-duplicates` posts the current record and lists stored
matches with their scores.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm run build
```

## License

Dual-licensed under MIT OR Apache-2.0.
