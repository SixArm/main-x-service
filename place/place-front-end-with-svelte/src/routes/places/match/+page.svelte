<!--
  Match check (route "/places/match") — ad-hoc matching tool: enter a
  partial place (name, address, optional geo, threshold) and see ranked
  candidate matches without creating anything.

  Local $state:
    - name / threshold / address / geo — the MatchRequest fields.
    - useGeo                           — whether geo is included in the request.
    - results / error / loading        — response and request status.
-->
<script lang="ts">
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import PostalAddressInput from "$lib/components/PostalAddressInput.svelte";
    import GeoCoordinatesInput from "$lib/components/GeoCoordinatesInput.svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import { blankPostalAddress } from "$lib/api/types.js";
    import type { GeoCoordinates, MatchRequest, MatchResult, PostalAddress } from "$lib/api/types.js";

    const repo = PlaceRepository.withFetch();

    let name = $state("");
    let threshold = $state(0.7);
    let address = $state<PostalAddress>(blankPostalAddress());
    let geo = $state<GeoCoordinates>({ latitude: 0, longitude: 0, elevation: null });
    let useGeo = $state(false);

    let results = $state<MatchResult[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Build and send the match request; geo is only included when opted in.
    async function runMatch(e: SubmitEvent) {
        e.preventDefault();
        loading = true;
        error = null;
        try {
            const req: MatchRequest = { name, address, threshold };
            if (useGeo) req.geo = geo;
            results = await repo.match(req);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            results = [];
        } finally {
            loading = false;
        }
    }
</script>

<svelte:head><title>Match check · Place Service</title></svelte:head>

<header><h1>Match check</h1></header>

<section class="surface stack">
    <form onsubmit={runMatch} class="stack">
        <FieldRow>
            <LabeledField label="Name" for="m-name" required><input id="m-name" bind:value={name} required /></LabeledField>
            <LabeledField label="Threshold" for="m-threshold" hint="0.0 – 1.0">
                <input id="m-threshold" type="number" step="0.05" min="0" max="1" bind:value={threshold} />
            </LabeledField>
        </FieldRow>
        <PostalAddressInput bind:address />
        <label class="row small">
            <input type="checkbox" bind:checked={useGeo} /> Include geo coordinates
        </label>
        {#if useGeo}
            <GeoCoordinatesInput bind:geo />
        {/if}
        {#if error}<div class="banner error">{error}</div>{/if}
        <button type="submit" class="button primary" disabled={loading}>
            {loading ? "Matching…" : "Find matches"}
        </button>
    </form>
</section>

{#if results.length > 0}
    <MatchResultsList {results} />
{/if}
