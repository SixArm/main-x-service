<!--
  Place detail (route "/places/[id]") — read-only view of one place with
  Edit / Audit links and a soft-delete action. Sections (address, geo,
  identifiers, opening hours, amenities) render only when present.

  Local $state:
    - place             — the fetched record (null until loaded).
    - error / loading   — request status.
  Derived:
    - id                — route param `[id]` (the place id).
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import type { Place } from "$lib/api/types.js";

    const repo = PlaceRepository.withFetch();
    let place = $state<Place | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    // Route param; `as string` because SvelteKit types params as optional.
    const id = $derived(page.params.id as string);

    onMount(async () => {
        try {
            place = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Soft-delete behind a confirm() guard, then return to the list.
    async function handleDelete() {
        if (!confirm("Soft-delete this place? This cannot be undone via the UI.")) return;
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
        return typeof p.place_type === "string" ? p.place_type : `Other: ${p.place_type.Other}`;
    }
</script>

<svelte:head><title>Place · {id}</title></svelte:head>

{#if loading}
    <p class="muted">Loading…</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if place}
    <header class="row" style="justify-content: space-between">
        <h1>{place.name}</h1>
        <div class="row">
            <a href={`/places/${id}/edit`} class="button">Edit</a>
            <a href={`/places/${id}/audit`} class="button">Audit</a>
            <button class="button danger" onclick={handleDelete}>Delete</button>
        </div>
    </header>

    <section class="surface stack">
        <h2>Identity</h2>
        <dl class="kv">
            <dt>ID</dt><dd><code>{place.id}</code></dd>
            <dt>Alternate name</dt><dd>{place.alternate_name ?? "—"}</dd>
            <dt>Type</dt><dd>{typeLabel(place)}</dd>
            <dt>Description</dt><dd>{place.description ?? "—"}</dd>
            <dt>URL</dt><dd>{#if place.url}<a href={place.url} target="_blank" rel="noopener">{place.url}</a>{:else}—{/if}</dd>
            <dt>Telephone</dt><dd>{place.telephone ?? "—"}</dd>
            <dt>GLN</dt><dd>{place.global_location_number ?? "—"}</dd>
            <dt>Branch code</dt><dd>{place.branch_code ?? "—"}</dd>
        </dl>
    </section>

    {#if place.address}
        <section class="surface stack">
            <h2>Address</h2>
            <p>
                <!-- Join only the populated address parts with commas. -->
                {[place.address.street_address, place.address.address_locality, place.address.address_region, place.address.postal_code, place.address.address_country]
                    .filter(Boolean)
                    .join(", ")}
            </p>
        </section>
    {/if}

    {#if place.geo}
        <section class="surface stack">
            <h2>Geo coordinates</h2>
            <dl class="kv">
                <dt>Latitude</dt><dd>{place.geo.latitude}</dd>
                <dt>Longitude</dt><dd>{place.geo.longitude}</dd>
                {#if place.geo.elevation != null}<dt>Elevation</dt><dd>{place.geo.elevation} m</dd>{/if}
            </dl>
        </section>
    {/if}

    {#if place.identifiers && place.identifiers.length > 0}
        <section class="surface stack">
            <h2>Identifiers</h2>
            <ul>
                {#each place.identifiers as identifier}
                    <li>
                        <strong>{typeof identifier.identifier_type === "string" ? identifier.identifier_type : `Custom: ${identifier.identifier_type.Custom}`}</strong>
                        <code>{identifier.value}</code>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if place.opening_hours && place.opening_hours.length > 0}
        <section class="surface stack">
            <h2>Opening hours</h2>
            <ul>
                {#each place.opening_hours as h}
                    <li><strong>{h.day_of_week}</strong> {h.opens}–{h.closes}</li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if place.amenity_features && place.amenity_features.length > 0}
        <section class="surface stack">
            <h2>Amenities</h2>
            <ul>
                {#each place.amenity_features as a}
                    <li>{a.name}{#if a.value}: {a.value}{/if}</li>
                {/each}
            </ul>
        </section>
    {/if}
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
    ul { margin: 0; padding-left: 1.25rem; }
</style>
