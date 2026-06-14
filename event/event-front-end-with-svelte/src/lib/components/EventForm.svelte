<!--
  EventForm — create/edit form for an Event. Wraps the reactive createForm
  helper with Event-specific validation (required name/start, end ≥ start,
  door ≤ start) and renders the field controls.

  Props:
    - initial (Event): starting value (a blank template for create, or the
      loaded record for edit).
    - submitLabel (string, default "Save"): primary button caption.
    - onsubmit ((event) => Promise<void>): persist handler; rejection surfaces
      via form.submitError.

  State:
    - form (FormState<Event>): the reactive form controller.
    - submitLabel / keywordsRaw / languagesRaw ($derived): derived display values.
-->
<script lang="ts">
    import type { Event } from "$lib/api/types.js";
    import {
        ATTENDANCE_MODES,
        EVENT_STATUSES,
        EVENT_TYPES,
    } from "$lib/api/types.js";
    import { createForm } from "$lib/forms/form.svelte.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";

    let props: {
        initial: Event;
        submitLabel?: string;
        onsubmit: (event: Event) => Promise<void>;
    } = $props();
    const submitLabel = $derived(props.submitLabel ?? "Save");

    // Normalize nullable array fields to [] so bindings/joins never hit undefined.
    function withDefaults(e: Event): Event {
        return {
            ...e,
            alternate_names: e.alternate_names ?? [],
            keywords: e.keywords ?? [],
            in_language: e.in_language ?? [],
            location: e.location ?? [],
            organizers: e.organizers ?? [],
        };
    }

    // svelte-ignore state_referenced_locally
    const form = createForm<Event>({
        initial: withDefaults(props.initial),
        // Client-side validation mirroring the service's time-window rules.
        validate(value) {
            const errors: Record<string, string> = {};
            if (!value.name.trim()) errors.name = "Required";
            if (!value.start_date) errors.start_date = "Required";
            // End must not precede start.
            if (value.start_date && value.end_date && Date.parse(value.end_date) < Date.parse(value.start_date)) {
                errors.end_date = "End must be ≥ start";
            }
            // Doors must open no later than the event start.
            if (value.door_time && value.start_date && Date.parse(value.door_time) > Date.parse(value.start_date)) {
                errors.door_time = "Door time must be ≤ start";
            }
            return errors;
        },
        onSubmit: (value) => props.onsubmit(value),
    });

    // Render the string[] fields as comma-/space-joined text for the inputs.
    let keywordsRaw = $derived((form.value.keywords ?? []).join(", "));
    let languagesRaw = $derived((form.value.in_language ?? []).join(", "));

    // Parse the free-text keyword input back into a trimmed, de-blanked array.
    function updateKeywords(v: string) {
        form.value.keywords = v.split(/[,\n]/).map((s) => s.trim()).filter(Boolean);
    }
    // Languages split on commas/whitespace and are lowercased (ISO 639-1).
    function updateLanguages(v: string) {
        form.value.in_language = v.split(/[,\s]+/).map((s) => s.trim().toLowerCase()).filter(Boolean);
    }

    // Prevent native navigation and delegate to the form controller's submit.
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
        <LabeledField label="Event type" for="event-type">
            <select id="event-type" bind:value={form.value.event_type}>
                <option value={null}>—</option>
                {#each EVENT_TYPES as t}<option value={t}>{t}</option>{/each}
            </select>
        </LabeledField>
    </FieldRow>

    <FieldRow>
        <LabeledField label="Start" for="start" required error={form.errors.start_date} hint="ISO 8601 / RFC 3339">
            <input id="start" type="datetime-local" bind:value={form.value.start_date} required />
        </LabeledField>
        <LabeledField label="End" for="end" error={form.errors.end_date}>
            <input id="end" type="datetime-local" bind:value={form.value.end_date} />
        </LabeledField>
        <LabeledField label="Door time" for="door" error={form.errors.door_time}>
            <input id="door" type="datetime-local" bind:value={form.value.door_time} />
        </LabeledField>
    </FieldRow>

    <FieldRow>
        <LabeledField label="Status" for="status">
            <select id="status" bind:value={form.value.event_status}>
                <option value={null}>—</option>
                {#each EVENT_STATUSES as s}<option value={s}>{s}</option>{/each}
            </select>
        </LabeledField>
        <LabeledField label="Attendance mode" for="mode">
            <select id="mode" bind:value={form.value.event_attendance_mode}>
                <option value={null}>—</option>
                {#each ATTENDANCE_MODES as m}<option value={m}>{m}</option>{/each}
            </select>
        </LabeledField>
        <LabeledField label="Time zone" for="tz" hint="IANA, e.g. America/Los_Angeles">
            <input id="tz" bind:value={form.value.time_zone} />
        </LabeledField>
        <LabeledField label="All day" for="all-day">
            <select id="all-day" bind:value={form.value.all_day}>
                <option value={false}>No</option>
                <option value={true}>Yes</option>
            </select>
        </LabeledField>
    </FieldRow>

    <LabeledField label="Description" for="desc">
        <textarea id="desc" rows={3} bind:value={form.value.description}></textarea>
    </LabeledField>

    <FieldRow>
        <LabeledField label="URL" for="url">
            <input id="url" type="url" bind:value={form.value.url} />
        </LabeledField>
        <LabeledField label="Duration" for="duration" hint="ISO 8601, e.g. PT1H30M">
            <input id="duration" bind:value={form.value.duration} />
        </LabeledField>
    </FieldRow>

    <FieldRow>
        <LabeledField label="Max capacity (total)" for="cap-total">
            <input id="cap-total" type="number" min="0" bind:value={form.value.maximum_attendee_capacity} />
        </LabeledField>
        <LabeledField label="Max physical" for="cap-physical">
            <input id="cap-physical" type="number" min="0" bind:value={form.value.maximum_physical_attendee_capacity} />
        </LabeledField>
        <LabeledField label="Max virtual" for="cap-virtual">
            <input id="cap-virtual" type="number" min="0" bind:value={form.value.maximum_virtual_attendee_capacity} />
        </LabeledField>
    </FieldRow>

    <FieldRow>
        <LabeledField label="Keywords" for="keywords" hint="Comma-separated">
            <input
                id="keywords"
                value={keywordsRaw}
                oninput={(e) => updateKeywords((e.target as HTMLInputElement).value)}
            />
        </LabeledField>
        <LabeledField label="Languages" for="langs" hint="ISO 639-1, e.g. en fr">
            <input
                id="langs"
                value={languagesRaw}
                oninput={(e) => updateLanguages((e.target as HTMLInputElement).value)}
            />
        </LabeledField>
    </FieldRow>

    {#if form.submitError}
        <div class="banner error">{form.submitError}</div>
    {/if}

    <div class="row">
        <button type="submit" class="button primary" disabled={form.submitting}>
            {form.submitting ? "Saving…" : submitLabel}
        </button>
        <button type="button" class="button" onclick={() => form.reset()} disabled={form.submitting}>Reset</button>
    </div>
</form>
