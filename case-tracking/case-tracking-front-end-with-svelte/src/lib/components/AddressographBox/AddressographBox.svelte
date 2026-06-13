<script lang="ts">
    // AddressographBox component
    //
    // The NHS "addressograph" — the patient-identity label block stuck on
    // paper case notes. Models the Lily Design System AddressographBox.
    // Headless: structure + ARIA only; styling lives in the app CSS.
    //
    // Props:
    //   name        — string, required. Patient full name.
    //   nhsNumber   — string, required. Formatted NHS Number (XXX XXX XXXX).
    //   dateOfBirth — string | null. Display date of birth.
    //   gender      — string | null, optional.
    //   address     — string | null, optional.
    //   label       — string, accessible name for the region.
    //   class       — string, optional.
    //
    // Claude rules:
    //   - Headless: no CSS. NHS Number carries the `.nhs-number` class so
    //     the app stylesheet renders it monospaced/bold like everywhere else.

    import UnitedKingdomNationalHealthServiceNumberView from '$lib/components/UnitedKingdomNationalHealthServiceNumberView/UnitedKingdomNationalHealthServiceNumberView.svelte';

    let {
        name,
        nhsNumber,
        dateOfBirth = null,
        gender = null,
        address = null,
        label = 'Patient addressograph',
        class: className = ''
    }: {
        name: string;
        nhsNumber: string;
        dateOfBirth?: string | null;
        gender?: string | null;
        address?: string | null;
        label?: string;
        class?: string;
    } = $props();
</script>

<section class={`addressograph-box ${className}`} aria-label={label}>
    <p class="addressograph-name">{name}</p>
    <dl class="addressograph-fields">
        <div>
            <dt>NHS No.</dt>
            <dd>
                <UnitedKingdomNationalHealthServiceNumberView
                    class="nhs-number"
                    label="NHS Number"
                    value={nhsNumber}
                />
            </dd>
        </div>
        <div>
            <dt>D.O.B.</dt>
            <dd>{dateOfBirth ?? '—'}</dd>
        </div>
        {#if gender}
            <div>
                <dt>Sex</dt>
                <dd>{gender}</dd>
            </div>
        {/if}
        {#if address}
            <div>
                <dt>Address</dt>
                <dd>{address}</dd>
            </div>
        {/if}
    </dl>
</section>
