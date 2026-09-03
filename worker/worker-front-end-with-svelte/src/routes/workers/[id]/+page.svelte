<!--
  Worker detail (route "/workers/[id]") — read-only view of one worker with
  links to edit/audit and a soft-delete action. Sections render only when
  their data is present. A masked-view toggle re-fetches through
  GET /api/workers/{id}/masked (T-19) instead of the plain record.

  $state:
    - worker — the loaded record, or null until/if loaded.
    - error — fetch/delete failure message.
    - loading — true until the initial fetch settles.
    - masked — whether the masked view is currently shown; re-fetches on
      toggle rather than masking client-side, so this always reflects the
      server's actual masking rules.

  $derived:
    - id — the worker id from the route params.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import LinksPanel from "$lib/components/LinksPanel.svelte";
    import type { Worker } from "$lib/api/types.js";
    import { t, tf } from "$lib/i18n.svelte.js";

    const repo = WorkerRepository.withFetch();
    let worker = $state<Worker | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);
    let masked = $state(false);

    // Route param `id`; cast since SvelteKit types params as string|undefined.
    const id = $derived(page.params.id as string);

    // Fetch the plain or masked record depending on `masked`, replacing
    // whatever is currently shown. Shared by the initial load and the
    // toggle handler so both go through one code path.
    async function load() {
        loading = true;
        error = null;
        try {
            worker = masked ? await repo.masked(id) : await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    }

    // Flip the toggle and re-fetch through the new endpoint. A dedicated
    // request per view, not client-side redaction — the server, not this
    // page, decides what counts as sensitive.
    function toggleMasked() {
        masked = !masked;
        void load();
    }

    // Load the worker on mount.
    onMount(load);

    // Soft-delete after a confirm prompt, then return to the list.
    async function handleDelete() {
        if (!confirm(t("detail.confirmDelete"))) return;
        try {
            await repo.softDelete(id);
            goto("/workers");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }
</script>

<svelte:head><title>{tf("detail.titleTab", { id })}</title></svelte:head>

{#if loading}
    <p class="muted">{t("common.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if worker}
    <header class="row" style="justify-content: space-between">
        <h1>{worker.name.given.join(" ")} {worker.name.family}</h1>
        <div class="row">
            <button class="button" aria-pressed={masked} onclick={toggleMasked}>
                {masked ? t("detail.showFull") : t("detail.showMasked")}
            </button>
            <a href={`/workers/${id}/edit`} class="button">{t("detail.edit")}</a
            >
            <a href={`/workers/${id}/audit`} class="button"
                >{t("detail.audit")}</a
            >
            <button class="button danger" onclick={handleDelete}
                >{t("detail.delete")}</button
            >
        </div>
    </header>

    {#if masked}
        <div class="banner" role="status">{t("detail.maskedNotice")}</div>
    {/if}

    <section class="surface stack">
        <h2>{t("detail.identity")}</h2>
        <dl class="kv">
            <dt>{t("detail.id")}</dt>
            <dd><code>{worker.id}</code></dd>
            <dt>{t("detail.active")}</dt>
            <dd>{worker.active ? t("detail.yes") : t("detail.no")}</dd>
            <dt>{t("detail.gender")}</dt>
            <dd>{worker.gender}</dd>
            <dt>{t("detail.birthDate")}</dt>
            <dd>{worker.birth_date ?? "—"}</dd>
            <dt>{t("detail.taxId")}</dt>
            <dd>{worker.tax_id ?? "—"}</dd>
            <dt>{t("detail.deceased")}</dt>
            <dd>
                {worker.deceased
                    ? (worker.deceased_datetime ?? t("detail.deceasedYes"))
                    : t("detail.deceasedNo")}
            </dd>
        </dl>
    </section>

    {#if worker.identifiers && worker.identifiers.length > 0}
        <section class="surface stack">
            <h2>{t("detail.identifiers")}</h2>
            <ul>
                {#each worker.identifiers as identifier}
                    <li>
                        <strong>{identifier.identifier_type}</strong>
                        <code>{identifier.value}</code>
                        <span class="muted small">@ {identifier.system}</span>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if worker.addresses && worker.addresses.length > 0}
        <section class="surface stack">
            <h2>{t("detail.addresses")}</h2>
            <ul>
                {#each worker.addresses as a}
                    <li>
                        {[
                            a.line1,
                            a.line2,
                            a.city,
                            a.state,
                            a.postal_code,
                            a.country,
                        ]
                            .filter(Boolean)
                            .join(", ")}
                        {#if a.use_type}<span class="muted small"
                                >({a.use_type})</span
                            >{/if}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if worker.telecom && worker.telecom.length > 0}
        <section class="surface stack">
            <h2>{t("detail.telecom")}</h2>
            <ul>
                {#each worker.telecom as t}
                    <li><strong>{t.system}</strong> {t.value}</li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if worker.emergency_contacts && worker.emergency_contacts.length > 0}
        <section class="surface stack">
            <h2>{t("detail.emergencyContacts")}</h2>
            <ul>
                {#each worker.emergency_contacts as ec}
                    <li>
                        <strong>{ec.name}</strong> — {ec.relationship}
                        {#if ec.is_primary}<span class="muted small"
                                >({t("detail.primary")})</span
                            >{/if}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    <!-- Cross-service edges (person / organization), not Worker.links. -->
    <LinksPanel workerId={id} />
{/if}

<style>
    .kv {
        display: grid;
        grid-template-columns: max-content 1fr;
        column-gap: 1rem;
        row-gap: 0.25rem;
    }
    dt {
        font-weight: 600;
    }
    dd {
        margin: 0;
    }
    ul {
        margin: 0;
        padding-left: 1.25rem;
    }
</style>
