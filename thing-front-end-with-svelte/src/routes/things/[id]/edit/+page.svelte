<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import ThingForm from "$lib/components/ThingForm.svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import type { Thing } from "$lib/api/types.js";

    const repo = ThingRepository.withFetch();
    let thing = $state<Thing | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

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

    async function handleSubmit(value: Thing) {
        await repo.update(id, value);
        goto(`/things/${id}`);
    }
</script>

<svelte:head><title>Edit thing · {id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Edit thing</h1>
    <a href={`/things/${id}`} class="button">Cancel</a>
</header>

{#if loading}
    <p class="muted">Loading…</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if thing}
    <section class="surface stack">
        <ThingForm initial={thing} submitLabel="Save changes" onsubmit={handleSubmit} />
    </section>
{/if}
