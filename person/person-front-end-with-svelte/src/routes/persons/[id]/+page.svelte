<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import type { Person } from "$lib/api/types.js";

    const repo = PersonRepository.withFetch();
    let person = $state<Person | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    const id = $derived(page.params.id as string);

    onMount(async () => {
        try {
            person = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    async function handleDelete() {
        if (!confirm("Soft-delete this person? This cannot be undone via the UI.")) return;
        try {
            await repo.softDelete(id);
            goto("/persons");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }
</script>

<svelte:head><title>Person · {id}</title></svelte:head>

{#if loading}
    <p class="muted">Loading…</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if person}
    <header class="row" style="justify-content: space-between">
        <h1>{person.name.given.join(" ")} {person.name.family}</h1>
        <div class="row">
            <a href={`/persons/${id}/edit`} class="button">Edit</a>
            <a href={`/persons/${id}/audit`} class="button">Audit</a>
            <button class="button danger" onclick={handleDelete}>Delete</button>
        </div>
    </header>

    <section class="surface stack">
        <h2>Identity</h2>
        <dl class="kv">
            <dt>ID</dt><dd><code>{person.id}</code></dd>
            <dt>Active</dt><dd>{person.active ? "Yes" : "No"}</dd>
            <dt>Gender</dt><dd>{person.gender}</dd>
            <dt>Birth date</dt><dd>{person.birth_date ?? "—"}</dd>
            <dt>Tax ID</dt><dd>{person.tax_id ?? "—"}</dd>
            <dt>Deceased</dt><dd>{person.deceased ? person.deceased_datetime ?? "yes" : "no"}</dd>
        </dl>
    </section>

    {#if person.identifiers && person.identifiers.length > 0}
        <section class="surface stack">
            <h2>Identifiers</h2>
            <ul>
                {#each person.identifiers as identifier}
                    <li>
                        <strong>{identifier.identifier_type}</strong>
                        <code>{identifier.value}</code>
                        <span class="muted small">@ {identifier.system}</span>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if person.addresses && person.addresses.length > 0}
        <section class="surface stack">
            <h2>Addresses</h2>
            <ul>
                {#each person.addresses as a}
                    <li>
                        {[a.line1, a.line2, a.city, a.state, a.postal_code, a.country].filter(Boolean).join(", ")}
                        {#if a.use_type}<span class="muted small">({a.use_type})</span>{/if}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if person.telecom && person.telecom.length > 0}
        <section class="surface stack">
            <h2>Telecom</h2>
            <ul>
                {#each person.telecom as t}
                    <li><strong>{t.system}</strong> {t.value}</li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if person.emergency_contacts && person.emergency_contacts.length > 0}
        <section class="surface stack">
            <h2>Emergency contacts</h2>
            <ul>
                {#each person.emergency_contacts as ec}
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
