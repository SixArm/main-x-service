<!--
  +page.svelte (/things/[id]) — Thing detail view.

  Purpose: loads one Thing by route id and renders its identity, identifiers,
  alternate names, same-as URLs, and images, with Edit / Audit / Delete
  actions. Delete is a soft delete and asks for confirmation. A
  masked-view toggle re-fetches through GET /api/things/{id}/masked
  (T-19) instead of the plain record.

  $state:
    - thing: the loaded record (null until fetched).
    - error / loading: request status.
    - masked: whether the masked view is currently shown; re-fetches on
      toggle rather than masking client-side, so this always reflects
      the server's actual masking rules.

  Reactive notes: `id` is $derived from page.params; the record loads in
  onMount. handleDelete navigates back to the list on success.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import type { Thing } from "$lib/api/types.js";
    import { t, translate } from "$lib/i18n.svelte.js";

    const repo = ThingRepository.withFetch();
    let thing = $state<Thing | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);
    let masked = $state(false);

    // Route param; cast since SvelteKit types params as possibly-undefined.
    const id = $derived(page.params.id as string);

    // Fetch the plain or masked record depending on `masked`, replacing
    // whatever is currently shown. Shared by the initial load and the
    // toggle handler so both go through one code path.
    async function load() {
        loading = true;
        error = null;
        try {
            thing = masked ? await repo.masked(id) : await repo.get(id);
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

    onMount(load);

    // Soft-delete the Thing after explicit confirmation, then return to list.
    async function handleDelete() {
        if (!confirm(t("detail.confirmDelete"))) return;
        try {
            await repo.softDelete(id);
            goto("/things");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    // Render an identifier's scheme as a label, handling the Custom variant.
    // The conditional type extracts the element type of the identifiers array.
    function identifierLabel(
        t: Thing["identifiers"] extends (infer U)[] | undefined ? U : never,
    ): string {
        return typeof t.property_id === "string"
            ? t.property_id
            : `${translate("detail.customPrefix")}${t.property_id.Custom}`;
    }
</script>

<svelte:head><title>Thing · {id}</title></svelte:head>

{#if loading}
    <p class="muted">{t("detail.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if thing}
    <header class="row" style="justify-content: space-between">
        <h1>{thing.name}</h1>
        <div class="row">
            <button class="button" aria-pressed={masked} onclick={toggleMasked}>
                {masked ? t("detail.showFull") : t("detail.showMasked")}
            </button>
            <a href={`/things/${id}/edit`} class="button">{t("detail.edit")}</a>
            <a href={`/things/${id}/audit`} class="button"
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
            <dd><code>{thing.id}</code></dd>
            <dt>{t("detail.additionalType")}</dt>
            <dd>
                {#if thing.additional_type}<a
                        href={thing.additional_type}
                        target="_blank"
                        rel="noopener">{thing.additional_type}</a
                    >{:else}—{/if}
            </dd>
            <dt>{t("detail.description")}</dt>
            <dd>{thing.description ?? "—"}</dd>
            <dt>{t("detail.disambiguating")}</dt>
            <dd>{thing.disambiguating_description ?? "—"}</dd>
            <dt>{t("detail.url")}</dt>
            <dd>
                {#if thing.url}<a
                        href={thing.url}
                        target="_blank"
                        rel="noopener">{thing.url}</a
                    >{:else}—{/if}
            </dd>
            <dt>{t("detail.owner")}</dt>
            <dd>{thing.owner ?? "—"}</dd>
            <dt>{t("detail.mainEntityOfPage")}</dt>
            <dd>
                {#if thing.main_entity_of_page}<a
                        href={thing.main_entity_of_page}
                        target="_blank"
                        rel="noopener">{thing.main_entity_of_page}</a
                    >{:else}—{/if}
            </dd>
        </dl>
    </section>

    {#if thing.identifiers && thing.identifiers.length > 0}
        <section class="surface stack">
            <h2>{t("detail.identifiers")}</h2>
            <ul>
                {#each thing.identifiers as identifier}
                    <li>
                        <strong>{identifierLabel(identifier)}</strong>
                        <code>{identifier.value}</code>
                        {#if identifier.url}<a
                                href={identifier.url}
                                target="_blank"
                                rel="noopener"
                                class="small">↗</a
                            >{/if}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if thing.alternate_names && thing.alternate_names.length > 0}
        <section class="surface stack">
            <h2>{t("detail.alternateNames")}</h2>
            <ul>
                {#each thing.alternate_names as alt}<li>{alt}</li>{/each}
            </ul>
        </section>
    {/if}

    {#if thing.same_as && thing.same_as.length > 0}
        <section class="surface stack">
            <h2>{t("detail.sameAs")}</h2>
            <ul>
                {#each thing.same_as as href}<li>
                        <a {href} target="_blank" rel="noopener">{href}</a>
                    </li>{/each}
            </ul>
        </section>
    {/if}

    {#if thing.images && thing.images.length > 0}
        <section class="surface stack">
            <h2>{t("detail.images")}</h2>
            <ul>
                {#each thing.images as src}<li>
                        <a href={src} target="_blank" rel="noopener">{src}</a>
                    </li>{/each}
            </ul>
        </section>
    {/if}
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
