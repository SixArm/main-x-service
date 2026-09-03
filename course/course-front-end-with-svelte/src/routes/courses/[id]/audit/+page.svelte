<!--
  Course audit log (route "/courses/[id]/audit") — newest-first list of
  audit entries for one course, each with action, timestamp, optional
  actor, and an expandable JSON payload of the new values. Loads up to
  100 entries client-side on mount.

  Reactive state:
    - id ($derived) — route param.
    - entries / loading / error — fetch result and status.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { CourseRepository } from "$lib/api/courses.js";
    import type { AuditEntry } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    const repo = CourseRepository.withFetch();
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
    <h1>{t("audit.title")}</h1>
    <a href={`/courses/${id}`} class="button">{t("audit.backToCourse")}</a>
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
                        <span class="muted small"
                            >{new Date(entry.created_at).toLocaleString()}</span
                        >
                        {#if entry.user_id}<span class="muted small"
                                >{t("audit.by")} {entry.user_id}</span
                            >{/if}
                    </header>
                    {#if entry.new_values}
                        <details>
                            <summary class="small">{t("audit.payload")}</summary
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
