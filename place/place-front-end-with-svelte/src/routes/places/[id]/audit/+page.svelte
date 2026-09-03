<!--
  Place audit log (route "/places/[id]/audit") — lists this place's audit
  entries (newest first, up to 100), each with an expandable JSON payload.

  Local $state:
    - entries / error / loading — fetched audit entries and request status.
  Derived:
    - id — the route's place id.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import { t, translate } from "$lib/i18n.svelte.js";
    import type { AuditEntry } from "$lib/api/types.js";

    const repo = PlaceRepository.withFetch();
    let entries = $state<AuditEntry[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(true);
    const id = $derived(page.params.id as string);

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
    <h1>{t("auditLog.title")}</h1>
    <a href={`/places/${id}`} class="button">{t("auditLog.backToPlace")}</a>
</header>

<section class="surface stack">
    {#if loading}
        <p class="muted">{t("auditLog.loading")}</p>
    {:else if error}
        <div class="banner error">{error}</div>
    {:else if entries.length === 0}
        <p class="muted">{t("auditLog.none")}</p>
    {:else}
        <ol class="entries">
            {#each entries as entry}
                <li>
                    <header class="row">
                        <code>{entry.action}</code>
                        <span class="muted small"
                            >{new Date(entry.created_at).toLocaleString()}</span
                        >
                        {#if entry.user_id}<span class="muted small"
                                >{translate("auditLog.by").replace(
                                    "{user}",
                                    entry.user_id,
                                )}</span
                            >{/if}
                    </header>
                    {#if entry.new_values}
                        <details>
                            <summary class="small"
                                >{t("auditLog.payload")}</summary
                            >
                            <pre class="small">{JSON.stringify(
                                    entry.new_values,
                                    null,
                                    2,
                                )}</pre>
                        </details>
                    {/if}
                </li>
            {/each}
        </ol>
    {/if}
</section>

<style>
    .entries {
        list-style: decimal;
        padding-left: 1.5rem;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    pre {
        background: var(--mxi-color-bg);
        padding: 0.5rem;
        border-radius: var(--mxi-radius);
        overflow-x: auto;
    }
</style>
