// Pure, framework-free validation for the Thing create/edit form, extracted
// from ThingForm.svelte so it can be unit-tested without mounting a Svelte
// component. Implements FR-4 (spec/06-functional-requirements.md): `name`
// required; the URL-shaped fields must be absolute http(s) URLs when present.
import type { Thing } from "$lib/api/types.js";

/** Field name → human-readable validation message. */
export type FieldErrors = Record<string, string>;

/** Thing fields that, when present, must be absolute http(s) URLs (FR-4). */
const URL_FIELDS: [keyof Thing, string][] = [
    ["url", "URL"],
    ["additional_type", "Additional type"],
    ["main_entity_of_page", "Main entity of page"],
    ["subject_of", "Subject of"],
];

/** True when `v` is an absolute `http://` or `https://` URL. */
function isHttpUrl(v: string): boolean {
    return /^https?:\/\//i.test(v);
}

/**
 * Validate a Thing for the create/edit form (FR-4).
 *
 * @param value - The candidate Thing.
 * @returns A map of field → error message; empty when the value is valid.
 */
export function validateThing(value: Thing): FieldErrors {
    const errors: FieldErrors = {};
    // Name is the only hard-required Thing field.
    if (!value.name.trim()) errors.name = "Required";
    // URL-shaped fields must be absolute http(s) URLs when present.
    for (const [field, label] of URL_FIELDS) {
        const v = value[field];
        if (typeof v === "string" && v.length > 0 && !isHttpUrl(v)) {
            errors[field as string] = `${label} must start with http(s)://`;
        }
    }
    return errors;
}
