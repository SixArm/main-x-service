# case-front-end-with-svelte

Operator UI for the [Case Service](../case-service-rust-crate):
case **CRUD + matching** (governmental case management).

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA.

## Routes

| Route | Purpose |
|---|---|
| `/` | List cases |
| `/new` | Create |
| `/[pid]` | Detail + delete + check-duplicates |
| `/[pid]/edit` | Edit |

## Prerequisites

- Node 20+ and pnpm
- A running [Case Service](../case-service-rust-crate)

## Quick start

```bash
cp .env.example .env     # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                 # http://localhost:5173
```

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `PUBLIC_API_BASE_URL` | `http://localhost:5150` | Case service REST base URL. |

## How it works

The case record body **is** the `case_matcher::Case` shape (title,
case number, agency, case type, status, priority, opened date, subjects,
keywords, identifiers, sameAs, languages). The form edits these;
`check-duplicates` posts the current record and lists stored matches with
their scores.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm test          # vitest unit tests
pnpm test:e2e      # Playwright smoke tests (build + preview)
pnpm run build
```

## License

Dual-licensed under MIT OR Apache-2.0.
