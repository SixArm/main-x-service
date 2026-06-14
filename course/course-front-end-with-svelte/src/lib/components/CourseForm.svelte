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
    import { COURSE_STATUSES, EDUCATIONAL_LEVEL_OPTIONS } from "$lib/api/types.js";
    import { createForm } from "$lib/forms/form.svelte.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import CourseIdentifierInput from "./CourseIdentifierInput.svelte";

    let props: {
        initial: Course;
        submitLabel?: string;
        onsubmit: (course: Course) => Promise<void>;
    } = $props();
    const submitLabel = $derived(props.submitLabel ?? "Save");

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

    /**
     * Strip blank string fields so the wire shape doesn't ship
     * `url: ""` and trip the service's FR-25 scheme check (empty
     * strings fail `"".startsWith("http://")` → 422). Empty strings
     * on optional fields are a form-UI artefact, not a real value;
     * convert them to `undefined` before submit so the omitted-key
     * branch of the service's serde default fires. Identifier URLs
     * follow the same rule — `identifier.url = ""` would otherwise
     * trip the per-identifier FR-25 branch.
     */
    function normalizeForWire(c: Course): Course {
        const blankToUndef = <T,>(v: T): T | undefined =>
            typeof v === "string" && v.trim() === "" ? undefined : v;
        return {
            ...c,
            description: blankToUndef(c.description),
            disambiguating_description: blankToUndef(c.disambiguating_description),
            url: blankToUndef(c.url),
            license: blankToUndef(c.license),
            additional_type: blankToUndef(c.additional_type),
            course_code: blankToUndef(c.course_code),
            typical_age_range: blankToUndef(c.typical_age_range),
            time_required: blankToUndef(c.time_required),
            version: blankToUndef(c.version),
            audience: blankToUndef(c.audience),
            educational_use: blankToUndef(c.educational_use),
            identifiers: c.identifiers?.map((i) => ({
                ...i,
                url: blankToUndef(i.url),
                name: blankToUndef(i.name),
            })),
        };
    }

    // svelte-ignore state_referenced_locally
    const form = createForm<Course>({
        initial: withDefaults(props.initial),
        // Client-side mirror of the service's required/format/range rules,
        // so obvious errors are caught before the round-trip.
        validate(value) {
            const errors: Record<string, string> = {};
            if (!value.name.trim()) errors.name = "Required";
            // Each of these fields must be a fully-qualified http(s) URL
            // if present (the service enforces the same in FR-25).
            const urlFields: [keyof Course, string][] = [
                ["url", "URL"],
                ["additional_type", "Additional type"],
                ["license", "License URL"],
            ];
            for (const [field, label] of urlFields) {
                const v = value[field];
                if (typeof v === "string" && v.length > 0 && !/^https?:\/\//i.test(v)) {
                    errors[field as string] = `${label} must start with http(s)://`;
                }
            }
            if (typeof value.course_code === "string" && value.course_code.length > 100) {
                errors.course_code = "Max 100 chars";
            }
            if (typeof value.number_of_credits === "number" && value.number_of_credits < 0) {
                errors.number_of_credits = "Must be ≥ 0";
            }
            return errors;
        },
        onSubmit: (value) => props.onsubmit(normalizeForWire(value)),
    });

    // String views of the array fields for binding to text controls.
    // newline-joined for one-per-line textareas; comma-joined for inline lists.
    let alternateNamesJoined = $derived((form.value.alternate_names ?? []).join("\n"));
    let sameAsJoined = $derived((form.value.same_as ?? []).join("\n"));
    let keywordsJoined = $derived((form.value.keywords ?? []).join(", "));
    let teachesJoined = $derived((form.value.teaches ?? []).join("\n"));
    let availableLangJoined = $derived((form.value.available_language ?? []).join(", "));

    // Inverse of the *Joined deriveds: parse edited text back into the
    // array fields, trimming and dropping blank entries.
    function updateAlternateNames(value: string) {
        form.value.alternate_names = value.split("\n").map((s) => s.trim()).filter(Boolean);
    }
    function updateSameAs(value: string) {
        form.value.same_as = value.split("\n").map((s) => s.trim()).filter(Boolean);
    }
    function updateKeywords(value: string) {
        // Keywords accept either comma or newline as a separator.
        form.value.keywords = value.split(/[,\n]/).map((s) => s.trim()).filter(Boolean);
    }
    function updateTeaches(value: string) {
        form.value.teaches = value.split("\n").map((s) => s.trim()).filter(Boolean);
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
    function isStringLevel(l: Course["educational_level"]): l is Exclude<NonNullable<Course["educational_level"]>, { Custom: string }> {
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
        <LabeledField label="Name" for="name" required error={form.errors.name}>
            <input id="name" bind:value={form.value.name} required />
        </LabeledField>
        <LabeledField label="Course code" for="course-code" hint="Provider-scoped (e.g. CS101)" error={form.errors.course_code}>
            <input id="course-code" bind:value={form.value.course_code} maxlength="100" />
        </LabeledField>
        <LabeledField label="Status" for="status">
            <select id="status" bind:value={form.value.status}>
                {#each COURSE_STATUSES as s}<option value={s}>{s}</option>{/each}
            </select>
        </LabeledField>
    </FieldRow>

    <LabeledField label="Description" for="desc">
        <textarea id="desc" rows={3} bind:value={form.value.description}></textarea>
    </LabeledField>

    <FieldRow>
        <LabeledField label="URL" for="url" error={form.errors.url}>
            <input id="url" type="url" bind:value={form.value.url} />
        </LabeledField>
        <LabeledField label="License" for="license" error={form.errors.license}>
            <input id="license" type="url" bind:value={form.value.license} />
        </LabeledField>
        <LabeledField label="Number of credits" for="credits" error={form.errors.number_of_credits}>
            <input id="credits" type="number" min="0" bind:value={form.value.number_of_credits} />
        </LabeledField>
    </FieldRow>

    <FieldRow>
        <LabeledField label="Educational level" for="level">
            <!--
              Custom-level objects can't be represented in this plain
              <select>, so they render as the blank "—" option; the
              empty choice writes back null (cleared level).
            -->
            <select
                id="level"
                value={isStringLevel(form.value.educational_level) ? form.value.educational_level : ""}
                onchange={(e) => {
                    const v = (e.target as HTMLSelectElement).value;
                    form.value.educational_level = (v || null) as Course["educational_level"];
                }}
            >
                <option value="">—</option>
                {#each EDUCATIONAL_LEVEL_OPTIONS as l}<option value={l}>{l}</option>{/each}
            </select>
        </LabeledField>
        <LabeledField label="Typical age range" for="age" hint="e.g. 18-22">
            <input id="age" bind:value={form.value.typical_age_range} />
        </LabeledField>
        <LabeledField label="Time required" for="duration" hint="ISO 8601 (e.g. PT45H)">
            <input id="duration" bind:value={form.value.time_required} />
        </LabeledField>
    </FieldRow>

    <LabeledField label="Alternate names" for="alt-names" hint="One per line">
        <textarea
            id="alt-names"
            rows={3}
            value={alternateNamesJoined}
            oninput={(e) => updateAlternateNames((e.target as HTMLTextAreaElement).value)}
        ></textarea>
    </LabeledField>

    <LabeledField label="Keywords" for="keywords" hint="Comma- or newline-separated">
        <input
            id="keywords"
            value={keywordsJoined}
            oninput={(e) => updateKeywords((e.target as HTMLInputElement).value)}
        />
    </LabeledField>

    <LabeledField label="Teaches (competencies)" for="teaches" hint="One per line">
        <textarea
            id="teaches"
            rows={3}
            value={teachesJoined}
            oninput={(e) => updateTeaches((e.target as HTMLTextAreaElement).value)}
        ></textarea>
    </LabeledField>

    <LabeledField label="Available languages" for="langs" hint="BCP-47 codes (e.g. en fr de)">
        <input
            id="langs"
            value={availableLangJoined}
            oninput={(e) => updateAvailableLanguage((e.target as HTMLInputElement).value)}
        />
    </LabeledField>

    <LabeledField label="Same-as URLs" for="same-as" hint="Wikidata, OER catalog, etc. — one per line">
        <textarea
            id="same-as"
            rows={3}
            value={sameAsJoined}
            oninput={(e) => updateSameAs((e.target as HTMLTextAreaElement).value)}
        ></textarea>
    </LabeledField>

    <section class="stack">
        <h2 class="small">Identifiers</h2>
        <CourseIdentifierInput bind:identifiers={form.value.identifiers!} />
    </section>

    {#if form.submitError}<div class="banner error">{form.submitError}</div>{/if}

    <div class="row">
        <button type="submit" class="button primary" disabled={form.submitting}>
            {form.submitting ? "Saving…" : submitLabel}
        </button>
        <button type="button" class="button" onclick={() => form.reset()} disabled={form.submitting}>Reset</button>
    </div>
</form>
