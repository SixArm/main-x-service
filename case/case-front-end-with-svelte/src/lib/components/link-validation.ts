// Pure guards for the cross-service links panel, extracted from
// `LinksPanel.svelte` so the accept/reject matrix is unit-testable without
// mounting a component. It mirrors — deliberately, without replacing — the
// service-side `validate_edge` in `case-service-with-loco`
// (`src/controllers/links.rs`), which is the authority: this check only
// saves an obviously-doomed round trip and states the reason in the
// operator's own language.
//
// Like `merge-validation.ts`, it returns an i18n **key** rather than an
// English sentence, so the panel renders the message in the selected
// locale via `t` / `translate`.

import type { StringKey } from "$lib/i18n.svelte";

/**
 * A person `EntityRef` URN: the literal type token `person`, a colon, and
 * a canonical 8-4-4-4-12 hex UUID (`agents/share/cross-service-linking.md`
 * §3). The service parses `to_ref` with the `entity-ref` crate, which
 * requires exactly this, so a client-side shape check is honest rather
 * than merely optimistic.
 */
export const PERSON_REF_PATTERN =
  /^person:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * Whether `value` looks like a person `EntityRef` URN.
 *
 * A case may only ever assert `subject_of` → **person** (§9), so any
 * other entity type is wrong here even when it is a well-formed ref.
 *
 * @param value - Candidate URN, e.g. `person:0c4f1e2a-…`.
 * @returns `true` when it matches {@link PERSON_REF_PATTERN}.
 */
export function isPersonRef(value: string): boolean {
  return PERSON_REF_PATTERN.test(value.trim());
}

/**
 * Validate the "record the subject" form before issuing the POST.
 *
 * @param toRef - The person `EntityRef` URN the case is about.
 * @param confidence - Optional operator confidence; `null` when the field
 *   is left empty (the service then stores no confidence at all).
 * @returns An i18n key for the message to surface, or `null` when the
 *   input is acceptable to send.
 */
export function validateLink(
  toRef: string,
  confidence: number | null,
): StringKey | null {
  if (!isPersonRef(toRef)) return "links.invalidPersonRef";
  if (
    confidence !== null &&
    (Number.isNaN(confidence) || confidence < 0 || confidence > 1)
  ) {
    return "links.confidenceRange";
  }
  return null;
}
