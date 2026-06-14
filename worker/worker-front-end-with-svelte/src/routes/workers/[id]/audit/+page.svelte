<!--
  Worker audit log (route "/workers/[id]/audit") — ordered list of audit
  entries for one worker, each with an expandable JSON payload of the new
  values.

  $state:
    - entries — the AuditEntry rows (up to 100).
    - error — fetch failure message.
    - loading — true until the fetch settles.

  $derived:
    - id — the worker id from the route params.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import type { AuditEntry } from "$lib/api/types.js";

    const repo = WorkerRepository.withFetch();
    let entries = $state<AuditEntry[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(true);
    const id = $derived(page.params.id as string);

    // Fetch up to 100 audit entries for this worker on mount.
    onMount(async () => {
        try {
            entries = await repo.audit(id, 100);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });
</script>

<svelte:head><title>Audit · {id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Audit log</h1>
    <a href={`/workers/${id}`} class="button">Back to worker</a>
</header>

<section class="surface stack">
    {#if loading}
        <p class="muted">Loading…</p>
    {:else if error}
        <div class="banner error">{error}</div>
    {:else if entries.length === 0}
        <p class="muted">No audit entries.</p>
    {:else}
        <ol class="entries">
            {#each entries as entry}
                <li>
                    <header class="row">
                        <code>{entry.action}</code>
                        <span class="muted small">{new Date(entry.created_at).toLocaleString()}</span>
                        {#if entry.user_id}<span class="muted small">by {entry.user_id}</span>{/if}
                    </header>
                    {#if entry.new_values}
                        <details>
                            <summary class="small">Payload</summary>
                            <pre class="small">{JSON.stringify(entry.new_values, null, 2)}</pre>
                        </details>
                    {/if}
                </li>
            {/each}
        </ol>
    {/if}
</section>

<style>
    .entries { list-style: decimal; padding-left: 1.5rem; display: flex; flex-direction: column; gap: 0.5rem; }
    pre { background: var(--mxi-color-bg); padding: 0.5rem; border-radius: var(--mxi-radius); overflow-x: auto; }
</style>
