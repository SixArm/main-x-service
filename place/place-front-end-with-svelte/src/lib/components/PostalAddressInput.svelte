<!--
  PostalAddressInput — labelled inputs for the five PostalAddress parts,
  laid out as two responsive rows. Each input two-way binds into the
  parent's `address` via `$bindable` (no events emitted).

  $props:
    - address (PostalAddress, $bindable) — the address being edited.
    - prefix (string) — id prefix to keep label/input ids unique when more
      than one address form is on the page. Default "addr".
-->
<script lang="ts">
    import type { PostalAddress } from "$lib/api/types.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";

    let {
        address = $bindable(),
        prefix = "addr",
    }: {
        address: PostalAddress;
        prefix?: string;
    } = $props();
</script>

<FieldRow>
    <LabeledField label="Street" for={`${prefix}-street`}>
        <input id={`${prefix}-street`} bind:value={address.street_address} />
    </LabeledField>
    <LabeledField label="City / locality" for={`${prefix}-city`}>
        <input id={`${prefix}-city`} bind:value={address.address_locality} />
    </LabeledField>
</FieldRow>
<FieldRow>
    <LabeledField label="Region / state" for={`${prefix}-region`}>
        <input id={`${prefix}-region`} bind:value={address.address_region} />
    </LabeledField>
    <LabeledField label="Postal code" for={`${prefix}-postal`}>
        <input id={`${prefix}-postal`} bind:value={address.postal_code} />
    </LabeledField>
    <LabeledField label="Country" for={`${prefix}-country`} hint="ISO 3166 alpha-2">
        <input id={`${prefix}-country`} bind:value={address.address_country} maxlength="2" />
    </LabeledField>
</FieldRow>
