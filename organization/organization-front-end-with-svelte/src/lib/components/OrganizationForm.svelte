<!--
  OrganizationForm — shared create/edit form for an Organization.

  Decomposes the flat `Organization` payload into editable `$state`
  fields (scalars, comma-separated lists, an identifiers array, and an
  address group), then reassembles it in `build()` on submit. Used by
  both `/new` (empty initial) and `/[pid]/edit` (loaded record).

  $props:
    - initial:     Organization — seed values; read once, untracked.
    - submitLabel: string        — submit button text (default "Save").
    - onsubmit:    (org) => Promise<void> — callback prop (runes events);
                   the parent persists and navigates. Thrown errors are
                   caught and shown inline.

  $state: every editable field, plus `submitting` (button disable) and
  `error` (inline banner). No `$derived` — the payload is built lazily
  in `build()` rather than tracked.
-->
<script lang="ts">
    import { untrack } from "svelte";
    import { ALL_SCHEMES } from "$lib/api/types";
    import type { IdentifierScheme, OrgIdentifier, Organization } from "$lib/api/types";
    import { buildOrganization } from "$lib/api/build";
    import { t } from "$lib/i18n.svelte";

    let {
        initial,
        submitLabel,
        onsubmit,
    }: {
        initial: Organization;
        submitLabel?: string;
        onsubmit: (org: Organization) => Promise<void>;
    } = $props();

    // Submit button text; falls back to the translated generic "Save".
    const label = $derived(submitLabel ?? t("form.save"));

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
    // Identifiers (unit-variant schemes only). Drop any `Custom` ({...})
    // schemes, which this form's dropdown cannot represent.
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

    /** Append a blank identifier row (default scheme `Lei`). */
    function addIdentifier() {
        // Reassign (not mutate) so the `$state` array triggers reactivity.
        identifiers = [...identifiers, { scheme: "Lei", value: "" }];
    }
    /** Remove the identifier row at index `i`. */
    function removeIdentifier(i: number) {
        identifiers = identifiers.filter((_, idx) => idx !== i);
    }

    /**
     * Reassemble the editable fields into an `Organization` payload via
     * the shared, unit-tested {@link buildOrganization} (spec §8): blank
     * scalars -> `null`, comma lists -> arrays, all-or-nothing address,
     * empty identifier rows dropped.
     */
    function build(): Organization {
        return buildOrganization({
            name,
            legalName,
            url,
            jurisdiction,
            foundingDate,
            telephone,
            email,
            alternateNames,
            keywords,
            sameAs,
            identifiers,
            street,
            locality,
            region,
            postalCode,
            country,
        });
    }

    /**
     * Form submit handler: validates `name`, builds the payload, and
     * delegates persistence to the `onsubmit` prop. Keeps `submitting`
     * true for the round-trip to disable the button, and surfaces any
     * thrown error inline.
     */
    async function handleSubmit(event: SubmitEvent) {
        // SPA: never let the browser navigate/POST.
        event.preventDefault();
        error = null;
        // Client-side guard mirroring the server's required-name rule.
        if (name.trim().length === 0) {
            error = t("form.nameRequired");
            return;
        }
        submitting = true;
        try {
            await onsubmit(build());
        } catch (err) {
            error = err instanceof Error ? err.message : t("form.saveFailed");
        } finally {
            submitting = false;
        }
    }
</script>

<form class="stack" onsubmit={handleSubmit}>
    <label>{t("form.name")}<input type="text" bind:value={name} required /></label>
    <label>{t("form.legalName")}<input type="text" bind:value={legalName} /></label>
    <div class="row">
        <label>{t("form.url")}<input type="url" bind:value={url} /></label>
        <label>{t("form.jurisdiction")}<input type="text" bind:value={jurisdiction} placeholder="US" /></label>
        <label>{t("form.foundingDate")}<input type="text" bind:value={foundingDate} placeholder="1971 or 1971-06-30" /></label>
    </div>
    <label>{t("form.alternateNames")} <small>{t("form.commaSeparated")}</small><input type="text" bind:value={alternateNames} /></label>
    <label>{t("form.keywords")} <small>{t("form.commaSeparated")}</small><input type="text" bind:value={keywords} /></label>
    <label>{t("form.sameAs")} <small>{t("form.commaSeparated")}</small><input type="text" bind:value={sameAs} /></label>

    <fieldset class="stack">
        <legend>{t("form.address")}</legend>
        <label>{t("form.street")}<input type="text" bind:value={street} /></label>
        <div class="row">
            <label>{t("form.locality")}<input type="text" bind:value={locality} /></label>
            <label>{t("form.region")}<input type="text" bind:value={region} /></label>
            <label>{t("form.postalCode")}<input type="text" bind:value={postalCode} /></label>
            <label>{t("form.country")}<input type="text" bind:value={country} /></label>
        </div>
    </fieldset>

    <fieldset class="stack">
        <legend>{t("form.identifiers")}</legend>
        <!-- Keyed by index `i`: rows have no stable id and are only
             appended/removed at the end, so positional keys are fine. -->
        {#each identifiers as identifier, i (i)}
            <div class="row">
                <select bind:value={identifier.scheme}>
                    {#each ALL_SCHEMES as scheme (String(scheme))}
                        <option value={scheme as IdentifierScheme}>{scheme}</option>
                    {/each}
                </select>
                <input type="text" bind:value={identifier.value} placeholder={t("form.value")} />
                <button type="button" onclick={() => removeIdentifier(i)}>{t("form.remove")}</button>
            </div>
        {/each}
        <button type="button" onclick={addIdentifier}>{t("form.addIdentifier")}</button>
    </fieldset>

    <button class="button" type="submit" disabled={submitting}>
        {submitting ? t("form.saving") : label}
    </button>
    {#if error}<p class="banner" role="alert">{error}</p>{/if}
</form>
