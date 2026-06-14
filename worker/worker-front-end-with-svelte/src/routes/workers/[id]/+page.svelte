<!--
  Worker detail (route "/workers/[id]") — read-only view of one worker with
  links to edit/audit and a soft-delete action. Sections render only when
  their data is present.

  $state:
    - worker — the loaded record, or null until/if loaded.
    - error — fetch/delete failure message.
    - loading — true until the initial fetch settles.

  $derived:
    - id — the worker id from the route params.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import type { Worker } from "$lib/api/types.js";

    const repo = WorkerRepository.withFetch();
    let worker = $state<Worker | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    // Route param `id`; cast since SvelteKit types params as string|undefined.
    const id = $derived(page.params.id as string);

    // Load the worker on mount.
    onMount(async () => {
        try {
            worker = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Soft-delete after a confirm prompt, then return to the list.
    async function handleDelete() {
        if (!confirm("Soft-delete this worker? This cannot be undone via the UI.")) return;
        try {
            await repo.softDelete(id);
            goto("/workers");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }
</script>

<svelte:head><title>Worker · {id}</title></svelte:head>

{#if loading}
    <p class="muted">Loading…</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if worker}
    <header class="row" style="justify-content: space-between">
        <h1>{worker.name.given.join(" ")} {worker.name.family}</h1>
        <div class="row">
            <a href={`/workers/${id}/edit`} class="button">Edit</a>
            <a href={`/workers/${id}/audit`} class="button">Audit</a>
            <button class="button danger" onclick={handleDelete}>Delete</button>
        </div>
    </header>

    <section class="surface stack">
        <h2>Identity</h2>
        <dl class="kv">
            <dt>ID</dt><dd><code>{worker.id}</code></dd>
            <dt>Active</dt><dd>{worker.active ? "Yes" : "No"}</dd>
            <dt>Gender</dt><dd>{worker.gender}</dd>
            <dt>Birth date</dt><dd>{worker.birth_date ?? "—"}</dd>
            <dt>Tax ID</dt><dd>{worker.tax_id ?? "—"}</dd>
            <dt>Deceased</dt><dd>{worker.deceased ? worker.deceased_datetime ?? "yes" : "no"}</dd>
        </dl>
    </section>

    {#if worker.identifiers && worker.identifiers.length > 0}
        <section class="surface stack">
            <h2>Identifiers</h2>
            <ul>
                {#each worker.identifiers as identifier}
                    <li>
                        <strong>{identifier.identifier_type}</strong>
                        <code>{identifier.value}</code>
                        <span class="muted small">@ {identifier.system}</span>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if worker.addresses && worker.addresses.length > 0}
        <section class="surface stack">
            <h2>Addresses</h2>
            <ul>
                {#each worker.addresses as a}
                    <li>
                        {[a.line1, a.line2, a.city, a.state, a.postal_code, a.country].filter(Boolean).join(", ")}
                        {#if a.use_type}<span class="muted small">({a.use_type})</span>{/if}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if worker.telecom && worker.telecom.length > 0}
        <section class="surface stack">
            <h2>Telecom</h2>
            <ul>
                {#each worker.telecom as t}
                    <li><strong>{t.system}</strong> {t.value}</li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if worker.emergency_contacts && worker.emergency_contacts.length > 0}
        <section class="surface stack">
            <h2>Emergency contacts</h2>
            <ul>
                {#each worker.emergency_contacts as ec}
                    <li>
                        <strong>{ec.name}</strong> — {ec.relationship}
                        {#if ec.is_primary}<span class="muted small">(primary)</span>{/if}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
    ul { margin: 0; padding-left: 1.25rem; }
</style>
