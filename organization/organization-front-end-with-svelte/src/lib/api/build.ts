// Pure form->payload assembly for OrganizationForm.
//
// Extracted from OrganizationForm.svelte so the spec §8 core (comma-list
// splitting, blank->null clearing, all-or-nothing address assembly,
// dropping empty identifier rows) is unit-testable without mounting the
// component. The component holds the `$state` fields and calls
// `buildOrganization` on submit; this module holds no reactive state.

import type { OrgIdentifier, Organization, ScoredRef } from "./types";

/** The flat set of editable inputs the form gathers before assembly. */
export interface OrganizationFormInputs {
    /** Display name (required; trimmed). */
    name: string;
    /** Legal name; blank -> null. */
    legalName: string;
    /** Website URL; blank -> null. */
    url: string;
    /** ISO 3166 jurisdiction; blank -> null. */
    jurisdiction: string;
    /** Founding date (year or ISO date); blank -> null. */
    foundingDate: string;
    /** Telephone contact; blank -> null. */
    telephone: string;
    /** Email contact; blank -> null. */
    email: string;
    /** Comma-separated alternate names. */
    alternateNames: string;
    /** Comma-separated keywords. */
    keywords: string;
    /** Comma-separated same-as URLs. */
    sameAs: string;
    /** Identifier rows (unit-variant schemes only). */
    identifiers: OrgIdentifier[];
    /** Street address line; part of the all-or-nothing address group. */
    street: string;
    /** Locality / city. */
    locality: string;
    /** Region / state. */
    region: string;
    /** Postal / ZIP code. */
    postalCode: string;
    /** Country. */
    country: string;
}

/** Split a comma-separated input into a trimmed, non-empty list. */
export function splitList(s: string): string[] {
    return s
        .split(",")
        .map((x) => x.trim())
        .filter((x) => x.length > 0);
}

/** Trim a scalar input; collapse a blank one to `undefined`. */
export function blankToUndef(s: string): string | undefined {
    const t = s.trim();
    return t.length > 0 ? t : undefined;
}

/**
 * Reassemble the editable inputs into an `Organization` payload.
 * Blank scalars become `null` (explicit clear); list inputs become
 * arrays; the address group is emitted only if at least one part is
 * present; identifier rows with empty values are dropped (and their
 * values trimmed).
 */
export function buildOrganization(input: OrganizationFormInputs): Organization {
    // Address is all-or-nothing: only attach it if some part is filled.
    const addrFields = [
        input.street,
        input.locality,
        input.region,
        input.postalCode,
        input.country,
    ].map(blankToUndef);
    const hasAddress = addrFields.some((x) => x !== undefined);
    const org: Organization = { name: input.name.trim() };
    org.legal_name = blankToUndef(input.legalName) ?? null;
    org.url = blankToUndef(input.url) ?? null;
    org.jurisdiction = blankToUndef(input.jurisdiction) ?? null;
    org.founding_date = blankToUndef(input.foundingDate) ?? null;
    org.telephone = blankToUndef(input.telephone) ?? null;
    org.email = blankToUndef(input.email) ?? null;
    org.alternate_names = splitList(input.alternateNames);
    org.keywords = splitList(input.keywords);
    org.same_as = splitList(input.sameAs);
    org.identifiers = input.identifiers
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

/**
 * Drop the self-match from a list of scored duplicates: the detail page
 * always matches itself, so the record's own `pid` is excluded before
 * display (spec §6.6). Pure so the exclusion is testable without the
 * route.
 */
export function excludeSelf(hits: ScoredRef[], selfPid: string): ScoredRef[] {
    return hits.filter((h) => h.pid !== selfPid);
}
