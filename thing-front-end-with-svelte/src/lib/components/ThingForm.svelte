<script lang="ts">
    import type { Thing } from "$lib/api/types.js";
    import { createForm } from "$lib/forms/form.svelte.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import ThingIdentifierInput from "./ThingIdentifierInput.svelte";

    let props: {
        initial: Thing;
        submitLabel?: string;
        onsubmit: (thing: Thing) => Promise<void>;
    } = $props();
    const submitLabel = $derived(props.submitLabel ?? "Save");

    function withDefaults(t: Thing): Thing {
        return {
            ...t,
            alternate_names: t.alternate_names ?? [],
            identifiers: t.identifiers ?? [],
            images: t.images ?? [],
            same_as: t.same_as ?? [],
        };
    }

    // svelte-ignore state_referenced_locally
    const form = createForm<Thing>({
        initial: withDefaults(props.initial),
        validate(value) {
            const errors: Record<string, string> = {};
            if (!value.name.trim()) errors.name = "Required";
            const urlFields: [keyof Thing, string][] = [
                ["url", "URL"],
                ["additional_type", "Additional type"],
                ["main_entity_of_page", "Main entity of page"],
                ["subject_of", "Subject of"],
            ];
            for (const [field, label] of urlFields) {
                const v = value[field];
                if (typeof v === "string" && v.length > 0 && !/^https?:\/\//i.test(v)) {
                    errors[field as string] = `${label} must start with http(s)://`;
                }
            }
            return errors;
        },
        onSubmit: (value) => props.onsubmit(value),
    });

    let alternateNamesJoined = $derived((form.value.alternate_names ?? []).join("\n"));
    let sameAsJoined = $derived((form.value.same_as ?? []).join("\n"));

    function updateAlternateNames(value: string) {
        form.value.alternate_names = value.split("\n").map((s) => s.trim()).filter(Boolean);
    }
    function updateSameAs(value: string) {
        form.value.same_as = value.split("\n").map((s) => s.trim()).filter(Boolean);
    }

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
        <LabeledField label="Additional type" for="add-type" error={form.errors.additional_type} hint="schema.org subtype URL">
            <input id="add-type" type="url" bind:value={form.value.additional_type} placeholder="https://schema.org/Book" />
        </LabeledField>
    </FieldRow>

    <LabeledField label="Description" for="desc">
        <textarea id="desc" rows={3} bind:value={form.value.description}></textarea>
    </LabeledField>

    <LabeledField label="Disambiguating description" for="disambig" hint="Short distinguishing detail">
        <input id="disambig" bind:value={form.value.disambiguating_description} />
    </LabeledField>

    <FieldRow>
        <LabeledField label="URL" for="url" error={form.errors.url}>
            <input id="url" type="url" bind:value={form.value.url} />
        </LabeledField>
        <LabeledField label="Owner" for="owner">
            <input id="owner" bind:value={form.value.owner} />
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

    <LabeledField label="Same-as URLs" for="same-as" hint="Wikidata, Wikipedia, etc. — one per line">
        <textarea
            id="same-as"
            rows={3}
            value={sameAsJoined}
            oninput={(e) => updateSameAs((e.target as HTMLTextAreaElement).value)}
        ></textarea>
    </LabeledField>

    <section class="stack">
        <h2 class="small">Identifiers</h2>
        <ThingIdentifierInput bind:identifiers={form.value.identifiers!} />
    </section>

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
