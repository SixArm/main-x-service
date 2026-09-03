<!--
  GeoCoordinatesInput — three numeric inputs (lat / lon / elevation) bound
  to a GeoCoordinates object. Editing the inputs mutates the parent's `geo`
  in place via `$bindable`, so no events are emitted.

  $props:
    - geo (GeoCoordinates, $bindable) — the coordinates being edited.
    - errors (Record<string,string>) — keyed by `latitude`/`longitude`;
      surfaced under the matching field.
    - prefix (string) — id prefix so multiple instances keep unique
      label/input ids on the page. Default "geo".
-->
<script lang="ts">
    import type { GeoCoordinates } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";

    let {
        geo = $bindable(),
        errors = {},
        prefix = "geo",
    }: {
        geo: GeoCoordinates;
        errors?: Record<string, string>;
        prefix?: string;
    } = $props();
</script>

<FieldRow>
    <LabeledField
        label={t("geo.latitude_as_decimal_degrees")}
        for={`${prefix}-lat`}
        error={errors.latitude_as_decimal_degrees}
        hint={t("geo.latitudeHint")}
    >
        <input
            id={`${prefix}-lat`}
            type="number"
            step="0.0001"
            min="-90"
            max="90"
            bind:value={geo.latitude_as_decimal_degrees}
        />
    </LabeledField>
    <LabeledField
        label={t("geo.longitude_as_decimal_degrees")}
        for={`${prefix}-lon`}
        error={errors.longitude_as_decimal_degrees}
        hint={t("geo.longitudeHint")}
    >
        <input
            id={`${prefix}-lon`}
            type="number"
            step="0.0001"
            min="-180"
            max="180"
            bind:value={geo.longitude_as_decimal_degrees}
        />
    </LabeledField>
    <LabeledField
        label={t("geo.elevation_as_decimal_metres")}
        for={`${prefix}-elev`}
    >
        <input
            id={`${prefix}-elev`}
            type="number"
            step="1"
            bind:value={geo.elevation_as_decimal_metres}
        />
    </LabeledField>
</FieldRow>
