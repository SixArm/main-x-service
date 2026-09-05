<!--
  Person audit log (/persons/[id]/audit) — chronological audit trail for
  one person.

  Loads a page of audit entries on mount and lists each action with its
  timestamp, actor, and an expandable JSON payload of the new values.

  T-31: the service's audit endpoint has no `offset`/cursor parameter
  (only `limit`, newest-first) — confirmed against
  `person-service-with-loco/src/api/rest/handlers.rs`'s `AuditLogQuery`
  and `db/audit.rs::get_logs_for_entity`, neither of which accepts one.
  "Load more" therefore re-requests with a **larger `limit`** each time
  rather than passing an offset; because the endpoint is newest-first
  and un-paginated, a larger limit's response is always a superset whose
  prefix is byte-identical to the previous response, so replacing
  `entries` with it is visually indistinguishable from appending — the
  entries already on screen never move or change, only more appear
  below. `hasMore` is `true` only while the last fetch returned exactly
  as many rows as requested (fewer means the server has nothing older
  left).

  State:
    - entries — the audit entries (newest first from the API).
    - limit — the current request size; grows by PAGE_SIZE each "load more".
    - hasMore — whether another "load more" could return additional rows.
    - error / loading / loadingMore — request lifecycle.
    - id ($derived) — the person id from the route params.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { AuditEntry } from "$lib/api/types.js";

    /** Page size for the initial load and each "load more" click. */
    const PAGE_SIZE = 100;

    const repo = PersonRepository.withFetch();
    let entries = $state<AuditEntry[]>([]);
    let limit = $state(PAGE_SIZE);
    let hasMore = $state(true);
    let error = $state<string | null>(null);
    let loading = $state(true);
    let loadingMore = $state(false);
    const id = $derived(page.params.id as string);

    // Fetch the top `requestedLimit` entries and update state. A response
    // shorter than requested means the server had nothing more to give.
    async function load(requestedLimit: number): Promise<void> {
        const fetched = await repo.audit(id, requestedLimit);
        entries = fetched;
        limit = requestedLimit;
        hasMore = fetched.length >= requestedLimit;
    }

    // Load this person's audit history on mount (SSR disabled).
    onMount(async () => {
        try {
            await load(PAGE_SIZE);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    async function loadMore(): Promise<void> {
        loadingMore = true;
        try {
            await load(limit + PAGE_SIZE);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loadingMore = false;
        }
    }
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
        {#if hasMore}
            <button
                type="button"
                class="button"
                disabled={loadingMore}
                onclick={loadMore}
            >
                {loadingMore ? t("audit.loadingMore") : t("audit.loadMore")}
            </button>
        {/if}
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
