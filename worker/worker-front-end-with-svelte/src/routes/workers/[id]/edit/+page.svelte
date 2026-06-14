<!--
  Edit worker (route "/workers/[id]/edit") — loads the record, hands it to
  WorkerForm, and saves changes via PUT.

  $state:
    - worker — the loaded record used as the form's initial value.
    - error — load/save failure message.
    - loading — true until the initial fetch settles.

  $derived:
    - id — the worker id from the route params.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import WorkerForm from "$lib/components/WorkerForm.svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import type { Worker } from "$lib/api/types.js";

    const repo = WorkerRepository.withFetch();
    let worker = $state<Worker | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    const id = $derived(page.params.id as string);

    // Load the record to pre-fill the form.
    onMount(async () => {
        try {
            worker = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // WorkerForm submit handler: persist the update then go to the detail
    // page. Errors here propagate to WorkerForm's submit banner.
    async function handleSubmit(value: Worker) {
        await repo.update(id, value);
        goto(`/workers/${id}`);
    }
</script>

<svelte:head><title>Edit worker · {id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Edit worker</h1>
    <a href={`/workers/${id}`} class="button">Cancel</a>
</header>

{#if loading}
    <p class="muted">Loading…</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if worker}
    <section class="surface stack">
        <WorkerForm initial={worker} submitLabel="Save changes" onsubmit={handleSubmit} />
    </section>
{/if}
