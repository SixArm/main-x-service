<script lang="ts">
    import { untrack } from "svelte";
    import { ALL_SCHEMES } from "$lib/api/types";
    import type { IdentifierScheme, OrgIdentifier, Organization } from "$lib/api/types";

    let {
        initial,
        submitLabel = "Save",
        onsubmit,
    }: {
        initial: Organization;
        submitLabel?: string;
        onsubmit: (org: Organization) => Promise<void>;
    } = $props();

    // Seed the form once from `initial` (read without tracking).
    const seed = untrack(() => initial);

    // Scalar fields.
    let name = $state(seed.name ?? "");
    let legalName = $state(seed.legal_name ?? "");
    let url = $state(seed.url ?? "");
    let jurisdiction = $state(seed.jurisdiction ?? "");
    let foundingDate = $state(seed.founding_date ?? "");
    let telephone = $state(seed.telephone ?? "");
    let email = $state(seed.email ?? "");
    // Comma-separated lists.
    let alternateNames = $state((seed.alternate_names ?? []).join(", "));
    let keywords = $state((seed.keywords ?? []).join(", "));
    let sameAs = $state((seed.same_as ?? []).join(", "));
    // Identifiers (unit-variant schemes only).
    let identifiers = $state<OrgIdentifier[]>(
        (seed.identifiers ?? []).filter((i) => typeof i.scheme === "string"),
    );
    // Address.
    let street = $state(seed.address?.street_address ?? "");
    let locality = $state(seed.address?.locality ?? "");
    let region = $state(seed.address?.region ?? "");
    let postalCode = $state(seed.address?.postal_code ?? "");
    let country = $state(seed.address?.country ?? "");

    let submitting = $state(false);
    let error = $state<string | null>(null);

    function splitList(s: string): string[] {
        return s
            .split(",")
            .map((x) => x.trim())
            .filter((x) => x.length > 0);
    }

    function blankToUndef(s: string): string | undefined {
        const t = s.trim();
        return t.length > 0 ? t : undefined;
    }

    function addIdentifier() {
        identifiers = [...identifiers, { scheme: "Lei", value: "" }];
    }
    function removeIdentifier(i: number) {
        identifiers = identifiers.filter((_, idx) => idx !== i);
    }

    function build(): Organization {
        const addrFields = [street, locality, region, postalCode, country].map(blankToUndef);
        const hasAddress = addrFields.some((x) => x !== undefined);
        const org: Organization = { name: name.trim() };
        org.legal_name = blankToUndef(legalName) ?? null;
        org.url = blankToUndef(url) ?? null;
        org.jurisdiction = blankToUndef(jurisdiction) ?? null;
        org.founding_date = blankToUndef(foundingDate) ?? null;
        org.telephone = blankToUndef(telephone) ?? null;
        org.email = blankToUndef(email) ?? null;
        org.alternate_names = splitList(alternateNames);
        org.keywords = splitList(keywords);
        org.same_as = splitList(sameAs);
        org.identifiers = identifiers
            .filter((i) => i.value.trim().length > 0)
            .map((i) => ({ scheme: i.scheme, value: i.value.trim() }));
        if (hasAddress) {
            org.address = {
                street_address: addrFields[0] ?? null,
                locality: addrFields[1] ?? null,
                region: addrFields[2] ?? null,
                postal_code: addrFields[3] ?? null,
                country: addrFields[4] ?? null,
            };
        }
        return org;
    }

    async function handleSubmit(event: SubmitEvent) {
        event.preventDefault();
        error = null;
        if (name.trim().length === 0) {
            error = "Name is required.";
            return;
        }
        submitting = true;
        try {
            await onsubmit(build());
        } catch (err) {
            error = err instanceof Error ? err.message : "Save failed";
        } finally {
            submitting = false;
        }
    }
</script>

<form class="stack" onsubmit={handleSubmit}>
    <label>Name<input type="text" bind:value={name} required /></label>
    <label>Legal name<input type="text" bind:value={legalName} /></label>
    <div class="row">
        <label>Website URL<input type="url" bind:value={url} /></label>
        <label>Jurisdiction (ISO 3166)<input type="text" bind:value={jurisdiction} placeholder="US" /></label>
        <label>Founding date<input type="text" bind:value={foundingDate} placeholder="1971 or 1971-06-30" /></label>
    </div>
    <label>Alternate names <small>(comma-separated)</small><input type="text" bind:value={alternateNames} /></label>
    <label>Keywords <small>(comma-separated)</small><input type="text" bind:value={keywords} /></label>
    <label>Same-as URLs <small>(comma-separated)</small><input type="text" bind:value={sameAs} /></label>

    <fieldset class="stack">
        <legend>Address</legend>
        <label>Street<input type="text" bind:value={street} /></label>
        <div class="row">
            <label>Locality<input type="text" bind:value={locality} /></label>
            <label>Region<input type="text" bind:value={region} /></label>
            <label>Postal code<input type="text" bind:value={postalCode} /></label>
            <label>Country<input type="text" bind:value={country} /></label>
        </div>
    </fieldset>

    <fieldset class="stack">
        <legend>Identifiers</legend>
        {#each identifiers as identifier, i (i)}
            <div class="row">
                <select bind:value={identifier.scheme}>
                    {#each ALL_SCHEMES as scheme (String(scheme))}
                        <option value={scheme as IdentifierScheme}>{scheme}</option>
                    {/each}
                </select>
                <input type="text" bind:value={identifier.value} placeholder="value" />
                <button type="button" onclick={() => removeIdentifier(i)}>Remove</button>
            </div>
        {/each}
        <button type="button" onclick={addIdentifier}>+ Add identifier</button>
    </fieldset>

    <button class="button" type="submit" disabled={submitting}>
        {submitting ? "Saving…" : submitLabel}
    </button>
    {#if error}<p class="banner" role="alert">{error}</p>{/if}
</form>
