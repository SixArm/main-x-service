<!--
  Person detail (/persons/[id]) — read-only view of one person plus edit /
  audit links and a soft-delete action.

  Loads the record on mount from the route param and renders identity,
  identifiers, addresses, telecom, and emergency contacts (each section
  shown only when populated). Delete confirms, soft-deletes, then returns
  to the list.

  State:
    - person — the loaded record (null until loaded / on error).
    - error / loading — request lifecycle.
    - id ($derived) — the person id from the route params.
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

    // Reactive route param; stays current if the user navigates between ids.
    const id = $derived(page.params.id as string);

    // Load the record on mount (SSR is disabled, so no +page.ts load).
    onMount(async () => {
        try {
            person = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

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
            <a href={`/persons/${id}/edit`} class="button">{t("detail.edit")}</a>
            <a href={`/persons/${id}/audit`} class="button">{t("detail.audit")}</a>
            <button class="button danger" onclick={handleDelete}>{t("detail.delete")}</button>
        </div>
    </header>

    <section class="surface stack">
        <h2>{t("detail.identity")}</h2>
        <dl class="kv">
            <dt>{t("detail.id")}</dt><dd><code>{person.id}</code></dd>
            <dt>{t("detail.active")}</dt><dd>{person.active ? t("detail.yes") : t("detail.no")}</dd>
            <dt>{t("detail.gender")}</dt><dd>{person.gender}</dd>
            <dt>{t("detail.birthDate")}</dt><dd>{person.birth_date ?? t("merge.noRecord")}</dd>
            <dt>{t("detail.taxId")}</dt><dd>{person.tax_id ?? t("merge.noRecord")}</dd>
            <dt>{t("detail.deceased")}</dt><dd>{person.deceased ? person.deceased_datetime ?? t("detail.deceasedYes") : t("detail.deceasedNo")}</dd>
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
                        {[a.line1, a.line2, a.city, a.state, a.postal_code, a.country].filter(Boolean).join(", ")}
                        {#if a.use_type}<span class="muted small">({a.use_type})</span>{/if}
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
                        {#if ec.is_primary}<span class="muted small">({t("detail.primary")})</span>{/if}
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
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
    ul { margin: 0; padding-left: 1.25rem; }
</style>
