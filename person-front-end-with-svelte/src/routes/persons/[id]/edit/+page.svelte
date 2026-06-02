<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import PersonForm from "$lib/components/PersonForm.svelte";
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

    async function handleSubmit(value: Person) {
        await repo.update(id, value);
        goto(`/persons/${id}`);
    }
</script>

<svelte:head><title>Edit person · {id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Edit person</h1>
    <a href={`/persons/${id}`} class="button">Cancel</a>
</header>

{#if loading}
    <p class="muted">Loading…</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if person}
    <section class="surface stack">
        <PersonForm initial={person} submitLabel="Save changes" onsubmit={handleSubmit} />
    </section>
{/if}
