<!--
  CourseForm — the create/edit form for a Course. Drives a reactive
  `createForm` controller, validates URL/length/credit fields, and
  normalises the value before handing it to the parent's submit
  handler. Shared by the "new course" and "edit course" routes.

  $props:
    - initial: Course — seed value (blank `{ name: "" }` for create, the
      fetched course for edit).
    - submitLabel?: string — primary button text (default "Save").
    - onsubmit: (course) => Promise<void> — called with the wire-normalised
      course when validation passes; should create/update and may throw.

  Key reactive state:
    - form — createForm controller holding value/errors/submitting.
    - *Joined ($derived) — array fields flattened to textarea/​input strings;
      the update* helpers parse the edited text back into arrays.
-->
<script lang="ts">
    import type { Course } from "$lib/api/types.js";
    import {
        COURSE_STATUSES,
        EDUCATIONAL_LEVEL_OPTIONS,
    } from "$lib/api/types.js";
    import { createForm } from "$lib/forms/form.svelte.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import CourseIdentifierInput from "./CourseIdentifierInput.svelte";
    import { validateCourse, normalizeForWire } from "./courseFormValidate.js";
    import { t } from "$lib/i18n.svelte.js";

    let props: {
        initial: Course;
        submitLabel?: string;
        onsubmit: (course: Course) => Promise<void>;
    } = $props();
    const submitLabel = $derived(props.submitLabel ?? t("form.save"));

    // Coerce all optional array/status fields to concrete defaults so the
    // form can bind to them without null checks on every input.
    function withDefaults(c: Course): Course {
        return {
            ...c,
            alternate_names: c.alternate_names ?? [],
            identifiers: c.identifiers ?? [],
            image: c.image ?? [],
            same_as: c.same_as ?? [],
            keywords: c.keywords ?? [],
            teaches: c.teaches ?? [],
            available_language: c.available_language ?? [],
            status: c.status ?? "draft",
        };
    }

    // svelte-ignore state_referenced_locally
    const form = createForm<Course>({
        initial: withDefaults(props.initial),
        // Client-side mirror of the service's required/format/range rules
        // (extracted to courseFormValidate.ts so it is unit-testable).
        validate: validateCourse,
        onSubmit: (value) => props.onsubmit(normalizeForWire(value)),
    });

    // String views of the array fields for binding to text controls.
    // newline-joined for one-per-line textareas; comma-joined for inline lists.
    let alternateNamesJoined = $derived(
        (form.value.alternate_names ?? []).join("\n"),
    );
    let sameAsJoined = $derived((form.value.same_as ?? []).join("\n"));
    let keywordsJoined = $derived((form.value.keywords ?? []).join(", "));
    let teachesJoined = $derived((form.value.teaches ?? []).join("\n"));
    let availableLangJoined = $derived(
        (form.value.available_language ?? []).join(", "),
    );

    // Inverse of the *Joined deriveds: parse edited text back into the
    // array fields, trimming and dropping blank entries.
    function updateAlternateNames(value: string) {
        form.value.alternate_names = value
            .split("\n")
            .map((s) => s.trim())
            .filter(Boolean);
    }
    function updateSameAs(value: string) {
        form.value.same_as = value
            .split("\n")
            .map((s) => s.trim())
            .filter(Boolean);
    }
    function updateKeywords(value: string) {
        // Keywords accept either comma or newline as a separator.
        form.value.keywords = value
            .split(/[,\n]/)
            .map((s) => s.trim())
            .filter(Boolean);
    }
    function updateTeaches(value: string) {
        form.value.teaches = value
            .split("\n")
            .map((s) => s.trim())
            .filter(Boolean);
    }
    function updateAvailableLanguage(value: string) {
        // BCP-47 codes: split on any whitespace/comma and lowercase.
        form.value.available_language = value
            .split(/[,\s]+/)
            .map((s) => s.trim().toLowerCase())
            .filter(Boolean);
    }

    // Type guard: narrows educational_level to its enumerated string
    // variants (excludes the { Custom } object), so the <select> can
    // bind a plain string value.
    function isStringLevel(
        l: Course["educational_level"],
    ): l is Exclude<
        NonNullable<Course["educational_level"]>,
        { Custom: string }
    > {
        return typeof l === "string";
    }

    // Intercept native submit, prevent navigation, and run the form
    // controller's validate+submit pipeline.
    function handleSubmit(e: SubmitEvent) {
        e.preventDefault();
        void form.submit();
    }
</script>

<form onsubmit={handleSubmit} class="stack">
    <FieldRow>
        <LabeledField
            label={t("form.name")}
            for="name"
            required
            error={form.errors.name}
        >
            <input id="name" bind:value={form.value.name} required />
        </LabeledField>
        <LabeledField
            label={t("form.courseCode")}
            for="course-code"
            hint={t("form.courseCodeHint")}
            error={form.errors.course_code}
        >
            <input
                id="course-code"
                bind:value={form.value.course_code}
                maxlength="100"
            />
        </LabeledField>
        <LabeledField label={t("form.status")} for="status">
            <select id="status" bind:value={form.value.status}>
                {#each COURSE_STATUSES as s}<option value={s}>{s}</option
                    >{/each}
            </select>
        </LabeledField>
    </FieldRow>

    <LabeledField label={t("form.description")} for="desc">
        <textarea id="desc" rows={3} bind:value={form.value.description}
        ></textarea>
    </LabeledField>

    <FieldRow>
        <LabeledField label={t("form.url")} for="url" error={form.errors.url}>
            <input id="url" type="url" bind:value={form.value.url} />
        </LabeledField>
        <LabeledField
            label={t("form.license")}
            for="license"
            error={form.errors.license}
        >
            <input id="license" type="url" bind:value={form.value.license} />
        </LabeledField>
        <LabeledField
            label={t("form.numberOfCredits")}
            for="credits"
            error={form.errors.number_of_credits}
        >
            <input
                id="credits"
                type="number"
                min="0"
                bind:value={form.value.number_of_credits}
            />
        </LabeledField>
    </FieldRow>

    <FieldRow>
        <LabeledField label={t("form.educationalLevel")} for="level">
            <!--
              Custom-level objects can't be represented in this plain
              <select>, so they render as the blank "—" option; the
              empty choice writes back null (cleared level).
            -->
            <select
                id="level"
                value={isStringLevel(form.value.educational_level)
                    ? form.value.educational_level
                    : ""}
                onchange={(e) => {
                    const v = (e.target as HTMLSelectElement).value;
                    form.value.educational_level = (v ||
                        null) as Course["educational_level"];
                }}
            >
                <option value="">—</option>
                {#each EDUCATIONAL_LEVEL_OPTIONS as l}<option value={l}
                        >{l}</option
                    >{/each}
            </select>
        </LabeledField>
        <LabeledField
            label={t("form.typicalAgeRange")}
            for="age"
            hint={t("form.typicalAgeRangeHint")}
        >
            <input id="age" bind:value={form.value.typical_age_range} />
        </LabeledField>
        <LabeledField
            label={t("form.timeRequired")}
            for="duration"
            hint={t("form.timeRequiredHint")}
        >
            <input id="duration" bind:value={form.value.time_required} />
        </LabeledField>
    </FieldRow>

    <LabeledField
        label={t("form.alternateNames")}
        for="alt-names"
        hint={t("form.alternateNamesHint")}
    >
        <textarea
            id="alt-names"
            rows={3}
            value={alternateNamesJoined}
            oninput={(e) =>
                updateAlternateNames((e.target as HTMLTextAreaElement).value)}
        ></textarea>
    </LabeledField>

    <LabeledField
        label={t("form.keywords")}
        for="keywords"
        hint={t("form.keywordsHint")}
    >
        <input
            id="keywords"
            value={keywordsJoined}
            oninput={(e) =>
                updateKeywords((e.target as HTMLInputElement).value)}
        />
    </LabeledField>

    <LabeledField
        label={t("form.teaches")}
        for="teaches"
        hint={t("form.teachesHint")}
    >
        <textarea
            id="teaches"
            rows={3}
            value={teachesJoined}
            oninput={(e) =>
                updateTeaches((e.target as HTMLTextAreaElement).value)}
        ></textarea>
    </LabeledField>

    <LabeledField
        label={t("form.availableLanguages")}
        for="langs"
        hint={t("form.availableLanguagesHint")}
    >
        <input
            id="langs"
            value={availableLangJoined}
            oninput={(e) =>
                updateAvailableLanguage((e.target as HTMLInputElement).value)}
        />
    </LabeledField>

    <LabeledField
        label={t("form.sameAs")}
        for="same-as"
        hint={t("form.sameAsHint")}
    >
        <textarea
            id="same-as"
            rows={3}
            value={sameAsJoined}
            oninput={(e) =>
                updateSameAs((e.target as HTMLTextAreaElement).value)}
        ></textarea>
    </LabeledField>

    <section class="stack">
        <h2 class="small">{t("form.identifiers")}</h2>
        <CourseIdentifierInput bind:identifiers={form.value.identifiers!} />
    </section>

    {#if form.submitError}<div class="banner error">
            {form.submitError}
        </div>{/if}

    <div class="row">
        <button type="submit" class="button primary" disabled={form.submitting}>
            {form.submitting ? t("form.saving") : submitLabel}
        </button>
        <button
            type="button"
            class="button"
            onclick={() => form.reset()}
            disabled={form.submitting}>{t("form.reset")}</button
        >
    </div>
</form>
