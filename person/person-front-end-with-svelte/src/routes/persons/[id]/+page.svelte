<!--
  Person detail (/persons/[id]) — read-only view of one person plus edit /
  audit links and a soft-delete action.

  Loads the record on mount from the route param and renders identity,
  identifiers, addresses, telecom, and emergency contacts (each section
  shown only when populated). Delete confirms, soft-deletes, then returns
  to the list. A masked-view toggle re-fetches through
  `GET /api/persons/{id}/masked` (T-19) instead of the plain record, for
  an operator who wants to see what a masked/lower-privilege caller sees.

  State:
    - person — the loaded record (null until loaded / on error).
    - error / loading — request lifecycle.
    - id ($derived) — the person id from the route params.
    - masked — whether the masked view is currently shown; re-fetches
      on toggle rather than masking client-side, so this always reflects
      the server's actual masking rules.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import LinksPanel from "$lib/components/LinksPanel.svelte";
    import { t } from "$lib/i18n.svelte.js";
    import type { Person } from "$lib/api/types.js";

    const repo = PersonRepository.withFetch();
    let person = $state<Person | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);
    let masked = $state(false);

    // Reactive route param; stays current if the user navigates between ids.
    const id = $derived(page.params.id as string);

    // Fetch the plain or masked record depending on `masked`, replacing
    // whatever is currently shown. Shared by the initial load and the
    // toggle handler so both go through one code path.
    async function load() {
        loading = true;
        error = null;
        try {
            person = masked ? await repo.masked(id) : await repo.get(id);
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

    // Load the record on mount (SSR is disabled, so no +page.ts load).
    onMount(load);

    // Confirm, then soft-delete and return to the list. Errors stay on-page.
    async function handleDelete() {
        if (!confirm(t("detail.confirmDelete"))) return;
        try {
            await repo.softDelete(id);
            goto("/persons");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }
</script>

<svelte:head><title>{t("detail.head.title.prefix")} · {id}</title></svelte:head>

{#if loading}
    <p class="muted">{t("detail.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if person}
    <header class="row" style="justify-content: space-between">
        <h1>{person.name.given.join(" ")} {person.name.family}</h1>
        <div class="row">
            <button class="button" aria-pressed={masked} onclick={toggleMasked}>
                {masked ? t("detail.showFull") : t("detail.showMasked")}
            </button>
            <a href={`/persons/${id}/edit`} class="button">{t("detail.edit")}</a
            >
            <a href={`/persons/${id}/audit`} class="button"
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
            <dd><code>{person.id}</code></dd>
            <dt>{t("detail.active")}</dt>
            <dd>{person.active ? t("detail.yes") : t("detail.no")}</dd>
            <dt>{t("detail.gender")}</dt>
            <dd>{person.gender}</dd>
            <dt>{t("detail.birthDate")}</dt>
            <dd>{person.birth_date ?? t("merge.noRecord")}</dd>
            <dt>{t("detail.taxId")}</dt>
            <dd>{person.tax_id ?? t("merge.noRecord")}</dd>
            <dt>{t("detail.deceased")}</dt>
            <dd>
                {person.deceased
                    ? (person.deceased_datetime ?? t("detail.deceasedYes"))
                    : t("detail.deceasedNo")}
            </dd>
        </dl>
    </section>

    {#if person.identifiers && person.identifiers.length > 0}
        <section class="surface stack">
            <h2>{t("detail.identifiers")}</h2>
            <ul>
                {#each person.identifiers as identifier}
                    <li>
                        <strong>{identifier.identifier_type}</strong>
                        <code>{identifier.value}</code>
                        <span class="muted small">@ {identifier.system}</span>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if person.addresses && person.addresses.length > 0}
        <section class="surface stack">
            <h2>{t("detail.addresses")}</h2>
            <ul>
                {#each person.addresses as a}
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

    {#if person.telecom && person.telecom.length > 0}
        <section class="surface stack">
            <h2>{t("detail.telecom")}</h2>
            <ul>
                {#each person.telecom as t}
                    <li><strong>{t.system}</strong> {t.value}</li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if person.emergency_contacts && person.emergency_contacts.length > 0}
        <section class="surface stack">
            <h2>{t("detail.emergencyContacts")}</h2>
            <ul>
                {#each person.emergency_contacts as ec}
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

    <!-- Cross-service edges (person → worker / organization). Separate
         from `person.links` above, which is the within-entity merge
         relationship — see LinksPanel's header comment. -->
    <LinksPanel personId={id} />
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
