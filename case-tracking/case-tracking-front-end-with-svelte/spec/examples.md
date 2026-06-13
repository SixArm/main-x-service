# Examples

> Part of the [Svelte edition specification](index.md). The wiring
> pattern is in [architecture.md](architecture.md); the cache API in
> [cache-api.md](cache-api.md).

## Hit the API from a route loader

```ts
// src/routes/patients/+page.ts
import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const list = await api.patients.list({}, { fetch });
        cache.setPatients(list.items);
        return {};
    } catch (e) {
        error(503, (e as Error).message);
    }
}
```

## Read the cache reactively in a page

```svelte
<script lang="ts">
    import { cache } from '$lib/store/cache.svelte';
    const patients = $derived(cache.patients);
</script>

{#each patients as p (p.id)}
    <li>{p.nhsNumber} — {p.name} ({p.folderCount} folders)</li>
{/each}
```

## Mutate and handle validation errors

```ts
import { ApiError } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';

try {
    await cache.addFolder({ nhsNumber, title, cabinetId });
} catch (e) {
    if (e instanceof ApiError && e.status === 422) {
        const { errors } = e.body as { errors: Record<string, string> };
        // surface errors.nhs_number, errors.title, etc. on the form
    } else {
        // unknown failure — show a page-level Alert
    }
}
```

## Add a new route

To add `/reports`:

1. Add a use case in [routes.md](routes.md) explaining the trigger and
   outcome, and a row to its routes table.
2. Create `src/routes/reports/+page.ts` that calls `api.*` and pushes
   the result into the cache (or returns it directly via `data`).
3. Create `src/routes/reports/+page.svelte`.
4. Add a `NavigationMenu` link in `+layout.svelte`.
5. If new API surface is needed, propose it in the
   [Loco spec](../../case-tracker-service-with-rust/spec/routes.md) first and
   add a typed method to `src/lib/api/client.ts`.
