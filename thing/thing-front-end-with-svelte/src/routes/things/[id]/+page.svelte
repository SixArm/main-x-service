<!--
  +page.svelte (/things/[id]) — Thing detail view.

  Purpose: loads one Thing by route id and renders its identity, identifiers,
  alternate names, same-as URLs, and images, with Edit / Audit / Delete
  actions. Delete is a soft delete and asks for confirmation.

  $state:
    - thing: the loaded record (null until fetched).
    - error / loading: request status.

  Reactive notes: `id` is $derived from page.params; the record loads in
  onMount. handleDelete navigates back to the list on success.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import type { Thing } from "$lib/api/types.js";

    const repo = ThingRepository.withFetch();
    let thing = $state<Thing | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    // Route param; cast since SvelteKit types params as possibly-undefined.
    const id = $derived(page.params.id as string);

    onMount(async () => {
        try {
            thing = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Soft-delete the Thing after explicit confirmation, then return to list.
    async function handleDelete() {
        if (!confirm("Soft-delete this thing? This cannot be undone via the UI.")) return;
        try {
            await repo.softDelete(id);
            goto("/things");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    // Render an identifier's scheme as a label, handling the Custom variant.
    // The conditional type extracts the element type of the identifiers array.
    function identifierLabel(t: Thing["identifiers"] extends (infer U)[] | undefined ? U : never): string {
        return typeof t.property_id === "string" ? t.property_id : `Custom: ${t.property_id.Custom}`;
    }
</script>

<svelte:head><title>Thing · {id}</title></svelte:head>

{#if loading}
    <p class="muted">Loading…</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if thing}
    <header class="row" style="justify-content: space-between">
        <h1>{thing.name}</h1>
        <div class="row">
            <a href={`/things/${id}/edit`} class="button">Edit</a>
            <a href={`/things/${id}/audit`} class="button">Audit</a>
            <button class="button danger" onclick={handleDelete}>Delete</button>
        </div>
    </header>

    <section class="surface stack">
        <h2>Identity</h2>
        <dl class="kv">
            <dt>ID</dt><dd><code>{thing.id}</code></dd>
            <dt>Additional type</dt>
            <dd>{#if thing.additional_type}<a href={thing.additional_type} target="_blank" rel="noopener">{thing.additional_type}</a>{:else}—{/if}</dd>
            <dt>Description</dt><dd>{thing.description ?? "—"}</dd>
            <dt>Disambiguating</dt><dd>{thing.disambiguating_description ?? "—"}</dd>
            <dt>URL</dt><dd>{#if thing.url}<a href={thing.url} target="_blank" rel="noopener">{thing.url}</a>{:else}—{/if}</dd>
            <dt>Owner</dt><dd>{thing.owner ?? "—"}</dd>
            <dt>Main entity of page</dt>
            <dd>{#if thing.main_entity_of_page}<a href={thing.main_entity_of_page} target="_blank" rel="noopener">{thing.main_entity_of_page}</a>{:else}—{/if}</dd>
        </dl>
    </section>

    {#if thing.identifiers && thing.identifiers.length > 0}
        <section class="surface stack">
            <h2>Identifiers</h2>
            <ul>
                {#each thing.identifiers as identifier}
                    <li>
                        <strong>{identifierLabel(identifier)}</strong>
                        <code>{identifier.value}</code>
                        {#if identifier.url}<a href={identifier.url} target="_blank" rel="noopener" class="small">↗</a>{/if}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if thing.alternate_names && thing.alternate_names.length > 0}
        <section class="surface stack">
            <h2>Alternate names</h2>
            <ul>
                {#each thing.alternate_names as alt}<li>{alt}</li>{/each}
            </ul>
        </section>
    {/if}

    {#if thing.same_as && thing.same_as.length > 0}
        <section class="surface stack">
            <h2>Same-as (authoritative URLs)</h2>
            <ul>
                {#each thing.same_as as href}<li><a {href} target="_blank" rel="noopener">{href}</a></li>{/each}
            </ul>
        </section>
    {/if}

    {#if thing.images && thing.images.length > 0}
        <section class="surface stack">
            <h2>Images</h2>
            <ul>
                {#each thing.images as src}<li><a href={src} target="_blank" rel="noopener">{src}</a></li>{/each}
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
