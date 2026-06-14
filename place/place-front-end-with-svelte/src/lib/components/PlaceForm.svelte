<!--
  PlaceForm — the create/edit form for a Place. Wraps the reactive
  `createForm` helper, performs client-side validation, and renders the
  Place fields plus optional Address and Geo sub-forms. On submit it hands
  the validated value back to the parent via `onsubmit`; the parent owns
  the actual API call.

  $props:
    - initial (Place)       — seed value (blank place for create, fetched
                              record for edit).
    - submitLabel (string?) — primary button text. Default "Save".
    - onsubmit ((place) => Promise<void>) — async persist callback; thrown
                              errors surface as `form.submitError`.

  Local $state:
    - form         — createForm controller holding value/errors/submitting.
    - hasAddress   — whether the Address section is included (toggles null).
    - hasGeo       — whether the Geo section is included (toggles null).
-->
<script lang="ts">
    import type { Place, PlaceType } from "$lib/api/types.js";
    import { PLACE_TYPES, blankPostalAddress } from "$lib/api/types.js";
    import { createForm } from "$lib/forms/form.svelte.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import PostalAddressInput from "./PostalAddressInput.svelte";
    import GeoCoordinatesInput from "./GeoCoordinatesInput.svelte";

    let props: {
        initial: Place;
        submitLabel?: string;
        onsubmit: (place: Place) => Promise<void>;
    } = $props();
    const submitLabel = $derived(props.submitLabel ?? "Save");

    // createForm reads props.initial once at setup; the ignore silences the
    // "state referenced locally" hint since this is an intentional snapshot.
    // svelte-ignore state_referenced_locally
    const form = createForm<Place>({
        initial: props.initial,
        // Client-side validation mirroring the service's rules so the
        // operator gets immediate feedback before a 422 round-trip.
        validate(value) {
            const errors: Record<string, string> = {};
            if (!value.name.trim()) errors.name = "Required";
            if (value.geo) {
                // Only validate coords when the Geo section is present.
                if (value.geo.latitude < -90 || value.geo.latitude > 90) errors.latitude = "-90 to 90";
                if (value.geo.longitude < -180 || value.geo.longitude > 180) errors.longitude = "-180 to 180";
            }
            // GLN, when supplied, must be exactly 13 digits (GS1).
            if (value.global_location_number && !/^\d{13}$/.test(value.global_location_number)) {
                errors.gln = "GLN must be 13 digits";
            }
            return errors;
        },
        onSubmit: (value) => props.onsubmit(value),
    });

    // Section toggles seeded from whether the initial record had the data.
    let hasGeo = $state(Boolean(form.value.geo));
    let hasAddress = $state(Boolean(form.value.address));

    // Map the PlaceType union to the `<select>`'s string value. The open
    // `{ Other }` variant has no option, so it collapses to "" (— / none).
    function selectedType(): PlaceType | "" {
        const t = form.value.place_type;
        if (!t) return "";
        if (typeof t === "string") return t;
        return "";
    }

    // Write the selected option back, treating the empty option as null.
    function setType(value: string) {
        form.value.place_type = value ? (value as PlaceType) : null;
    }

    // Toggling the Address section swaps between a blank address and null
    // so an unchecked section sends `address: null` (not stale data).
    function toggleAddress(on: boolean) {
        hasAddress = on;
        form.value.address = on ? blankPostalAddress() : null;
    }

    // Same pattern for Geo: zeroed coords when on, null when off.
    function toggleGeo(on: boolean) {
        hasGeo = on;
        form.value.geo = on ? { latitude: 0, longitude: 0, elevation: null } : null;
    }

    // Suppress native submit and delegate to the form controller (which
    // validates, then calls onsubmit).
    function handleSubmit(e: SubmitEvent) {
        e.preventDefault();
        void form.submit();
    }
</script>

<form onsubmit={handleSubmit} class="stack">
    <FieldRow>
        <LabeledField label="Name" for="name" required error={form.errors.name}>
            <input id="name" bind:value={form.value.name} required />
        </LabeledField>
        <LabeledField label="Alternate name" for="alt-name">
            <input id="alt-name" bind:value={form.value.alternate_name} />
        </LabeledField>
        <LabeledField label="Place type" for="place-type">
            <select id="place-type" value={selectedType()} onchange={(e) => setType((e.target as HTMLSelectElement).value)}>
                <option value="">—</option>
                {#each PLACE_TYPES as t}
                    <option value={t}>{t}</option>
                {/each}
            </select>
        </LabeledField>
    </FieldRow>

    <LabeledField label="Description" for="desc">
        <textarea id="desc" rows={3} bind:value={form.value.description}></textarea>
    </LabeledField>

    <FieldRow>
        <LabeledField label="Telephone" for="phone">
            <input id="phone" type="tel" bind:value={form.value.telephone} />
        </LabeledField>
        <LabeledField label="Website" for="url">
            <input id="url" type="url" bind:value={form.value.url} />
        </LabeledField>
        <LabeledField label="GLN" for="gln" error={form.errors.gln} hint="13-digit Global Location Number">
            <input id="gln" bind:value={form.value.global_location_number} maxlength="13" />
        </LabeledField>
    </FieldRow>

    <section class="surface stack">
        <header class="row" style="justify-content: space-between">
            <h2 class="small">Address</h2>
            <label class="row small">
                <input type="checkbox" checked={hasAddress} onchange={(e) => toggleAddress((e.target as HTMLInputElement).checked)} />
                Include address
            </label>
        </header>
        {#if hasAddress && form.value.address}
            <PostalAddressInput bind:address={form.value.address} />
        {/if}
    </section>

    <section class="surface stack">
        <header class="row" style="justify-content: space-between">
            <h2 class="small">Geo coordinates</h2>
            <label class="row small">
                <input type="checkbox" checked={hasGeo} onchange={(e) => toggleGeo((e.target as HTMLInputElement).checked)} />
                Include coords
            </label>
        </header>
        {#if hasGeo && form.value.geo}
            <GeoCoordinatesInput bind:geo={form.value.geo} errors={form.errors} />
        {/if}
    </section>

    {#if form.submitError}
        <div class="banner error">{form.submitError}</div>
    {/if}

    <div class="row">
        <button type="submit" class="button primary" disabled={form.submitting}>
            {form.submitting ? "Saving…" : submitLabel}
        </button>
        <button type="button" class="button" onclick={() => form.reset()} disabled={form.submitting}>
            Reset
        </button>
    </div>
</form>
