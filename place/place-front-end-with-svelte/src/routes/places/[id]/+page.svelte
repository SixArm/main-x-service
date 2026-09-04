<!--
  Place detail (route "/places/[id]") — read-only view of one place with
  Edit / Audit links and a soft-delete action. Sections (address, geo,
  identifiers, opening hours, amenities) render only when present. A
  masked-view toggle re-fetches through GET /api/places/{id}/masked
  (T-19) instead of the plain record.

  Local $state:
    - place             — the fetched record (null until loaded).
    - error / loading   — request status.
    - masked            — whether the masked view is currently shown;
      re-fetches on toggle rather than masking client-side, so this
      always reflects the server's actual masking rules.
  Derived:
    - id                — route param `[id]` (the place id).
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import { t, translate } from "$lib/i18n.svelte.js";
    import type { Place } from "$lib/api/types.js";

    const repo = PlaceRepository.withFetch();
    let place = $state<Place | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);
    let exporting = $state(false);
    let masked = $state(false);

    // Route param; `as string` because SvelteKit types params as optional.
    const id = $derived(page.params.id as string);

    // Fetch the plain or masked record depending on `masked`, replacing
    // whatever is currently shown. Shared by the initial load and the
    // toggle handler so both go through one code path.
    async function load() {
        loading = true;
        error = null;
        try {
            place = masked ? await repo.masked(id) : await repo.get(id);
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

    // Soft-delete behind a confirm() guard, then return to the list.
    async function handleDelete() {
        if (!confirm(translate("detail.confirmDelete"))) return;
        try {
            await repo.softDelete(id);
            goto("/places");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    // Display label for the place type; `{ Other }` → "Other: <value>".
    function typeLabel(p: Place): string {
        if (!p.place_type) return "—";
        return typeof p.place_type === "string"
            ? p.place_type
            : translate("detail.typeOther").replace(
                  "{value}",
                  p.place_type.Other,
              );
    }
    // GDPR export (T-20): fetch the service's export payload and hand it
    // to the browser as a downloaded JSON file — the payload shape is
    // service-defined (`exportGdpr` returns `unknown`), so this never
    // interprets it, only serializes and saves what came back. A Blob
    // object URL through a synthetic anchor is the plain-browser way to
    // save client-held data; the URL is revoked once the click has fired.
    async function handleExportGdpr() {
        exporting = true;
        error = null;
        try {
            const data = await repo.exportGdpr(id);
            const blob = new Blob([JSON.stringify(data, null, 2)], {
                type: "application/json",
            });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = `place-${id}-export.json`;
            document.body.appendChild(a);
            a.click();
            a.remove();
            URL.revokeObjectURL(url);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            exporting = false;
        }
    }
</script>

<svelte:head><title>Place · {id}</title></svelte:head>

{#if loading}
    <p class="muted">{t("detail.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if place}
    <header class="row" style="justify-content: space-between">
        <h1>{place.name}</h1>
        <div class="row">
            <button class="button" aria-pressed={masked} onclick={toggleMasked}>
                {masked ? t("detail.showFull") : t("detail.showMasked")}
            </button>
            <a href={`/places/${id}/edit`} class="button">{t("detail.edit")}</a>
            <a href={`/places/${id}/audit`} class="button"
                >{t("detail.audit")}</a
            >
            <button
                class="button"
                onclick={handleExportGdpr}
                disabled={exporting}
            >
                {exporting ? t("detail.exportingGdpr") : t("detail.exportGdpr")}
            </button>
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
            <dd><code>{place.id}</code></dd>
            <dt>{t("detail.alternateName")}</dt>
            <dd>{place.alternate_name ?? "—"}</dd>
            <dt>{t("detail.type")}</dt>
            <dd>{typeLabel(place)}</dd>
            <dt>{t("detail.description")}</dt>
            <dd>{place.description ?? "—"}</dd>
            <dt>{t("detail.url")}</dt>
            <dd>
                {#if place.url}<a
                        href={place.url}
                        target="_blank"
                        rel="noopener">{place.url}</a
                    >{:else}—{/if}
            </dd>
            <dt>{t("detail.telephone")}</dt>
            <dd>{place.telephone ?? "—"}</dd>
            <dt>{t("detail.gln")}</dt>
            <dd>{place.global_location_number ?? "—"}</dd>
            <dt>{t("detail.branchCode")}</dt>
            <dd>{place.branch_code ?? "—"}</dd>
        </dl>
    </section>

    {#if place.address}
        <section class="surface stack">
            <h2>{t("detail.address")}</h2>
            <p>
                <!-- Join only the populated address parts with commas. -->
                {[
                    place.address.street_address,
                    place.address.address_locality,
                    place.address.address_region,
                    place.address.postal_code,
                    place.address.address_country,
                ]
                    .filter(Boolean)
                    .join(", ")}
            </p>
        </section>
    {/if}

    {#if place.geo}
        <section class="surface stack">
            <h2>{t("detail.geo")}</h2>
            <dl class="kv">
                <dt>{t("detail.latitude_as_decimal_degrees")}</dt>
                <dd>{place.geo.latitude_as_decimal_degrees}</dd>
                <dt>{t("detail.longitude_as_decimal_degrees")}</dt>
                <dd>{place.geo.longitude_as_decimal_degrees}</dd>
                {#if place.geo.elevation_as_decimal_metres != null}<dt>
                        {t("detail.elevation_as_decimal_metres")}
                    </dt>
                    <dd>{place.geo.elevation_as_decimal_metres} m</dd>{/if}
            </dl>
        </section>
    {/if}

    {#if place.identifiers && place.identifiers.length > 0}
        <section class="surface stack">
            <h2>{t("detail.identifiers")}</h2>
            <ul>
                {#each place.identifiers as identifier}
                    <li>
                        <strong
                            >{typeof identifier.identifier_type === "string"
                                ? identifier.identifier_type
                                : translate("detail.identifierCustom").replace(
                                      "{value}",
                                      identifier.identifier_type.Custom,
                                  )}</strong
                        >
                        <code>{identifier.value}</code>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if place.opening_hours && place.opening_hours.length > 0}
        <section class="surface stack">
            <h2>{t("detail.openingHours")}</h2>
            <ul>
                {#each place.opening_hours as h}
                    <li>
                        <strong>{h.day_of_week}</strong>
                        {h.opens}–{h.closes}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if place.amenity_features && place.amenity_features.length > 0}
        <section class="surface stack">
            <h2>{t("detail.amenities")}</h2>
            <ul>
                {#each place.amenity_features as a}
                    <li>
                        {a.name}{#if a.value}: {a.value}{/if}
                    </li>
                {/each}
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
