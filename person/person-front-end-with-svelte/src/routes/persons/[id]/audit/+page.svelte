<!--
  Person audit log (/persons/[id]/audit) — chronological audit trail for
  one person.

  Loads up to 100 audit entries on mount and lists each action with its
  timestamp, actor, and an expandable JSON payload of the new values.

  State:
    - entries — the audit entries (newest first from the API).
    - error / loading — request lifecycle.
    - id ($derived) — the person id from the route params.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { AuditEntry } from "$lib/api/types.js";

    const repo = PersonRepository.withFetch();
    let entries = $state<AuditEntry[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(true);
    const id = $derived(page.params.id as string);

    // Load this person's audit history on mount (SSR disabled).
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

<svelte:head><title>{t("audit.head.title.prefix")}{id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("audit.title")}</h1>
    <a href={`/persons/${id}`} class="button">{t("audit.back")}</a>
</header>

<section class="surface stack">
    {#if loading}
        <p class="muted">{t("audit.loading")}</p>
    {:else if error}
        <div class="banner error">{error}</div>
    {:else if entries.length === 0}
        <p class="muted">{t("audit.noEntries")}</p>
    {:else}
        <ol class="entries">
            {#each entries as entry}
                <li>
                    <header class="row">
                        <code>{entry.action}</code>
                        <span class="muted small">{new Date(entry.created_at).toLocaleString()}</span>
                        {#if entry.user_id}<span class="muted small">{t("audit.by")} {entry.user_id}</span>{/if}
                    </header>
                    {#if entry.new_values}
                        <details>
                            <summary class="small">{t("audit.payload")}</summary>
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
