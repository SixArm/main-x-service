// Client-side rules for the cross-service links a person may originate.
//
// Mirrors `person-service-with-loco/src/api/rest/links.rs::validate_edge`
// and the `entity-ref` crate's `EntityRef` parser, so the operator learns
// the constraint from the form rather than from a 422. The server stays
// authoritative — this is a pre-flight courtesy, never a substitute for
// its answer.
//
// Pure and dependency-free (no Svelte, no fetch) so the accept/reject
// matrix is unit-testable exactly as the Rust side's is.

/**
 * The edge kinds person may originate, per the §9 registry.
 * `employed_by` is worker-originated and `subject_of` is
 * case-originated, so neither appears here.
 */
export const PERSON_LINK_KINDS = [
  "same_identity",
  "works_at",
  "member_of",
] as const;

/** One of {@link PERSON_LINK_KINDS}. */
export type PersonLinkKind = (typeof PERSON_LINK_KINDS)[number];

/**
 * The entity type each kind's target must be. `same_identity` is the
 * person↔worker identity backbone; the two affiliations point at an
 * organization.
 */
export const LINK_KIND_TARGET_TYPE: Record<PersonLinkKind, string> = {
  same_identity: "worker",
  works_at: "organization",
  member_of: "organization",
};

/** Why a `to_ref` was rejected before the request was sent. */
export type ToRefProblem = "required" | "malformed" | "wrong-type";

/** Whether `value` is one of the kinds person may originate. */
export function isPersonLinkKind(value: string): value is PersonLinkKind {
  return (PERSON_LINK_KINDS as readonly string[]).includes(value);
}

/** The entity type `kind`'s target must have. */
export function expectedTargetType(kind: PersonLinkKind): string {
  return LINK_KIND_TARGET_TYPE[kind];
}

/**
 * A worked example of the URN shape `kind` expects, for the input's
 * placeholder — e.g. `worker:<uuid>`. Language-neutral by construction,
 * so it is not part of the translation catalog.
 */
export function refPlaceholder(kind: PersonLinkKind): string {
  return `${expectedTargetType(kind)}:<uuid>`;
}

// Canonical UUID text form; the Rust side parses with `Uuid::parse_str`,
// which is stricter than a loose hex/dash match. Case-insensitive.
const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * Split an `EntityRef` URN into its parts, mirroring the Rust
 * `FromStr`: exactly one `:`, a non-empty type token, and a canonical
 * UUID id.
 *
 * @returns The parts, or `null` when the string is not a well-formed ref.
 */
export function parseEntityRef(
  ref: string,
): { entityType: string; id: string } | null {
  const separator = ref.indexOf(":");
  if (separator <= 0) return null;
  const entityType = ref.slice(0, separator);
  const id = ref.slice(separator + 1);
  if (id.includes(":") || !UUID_RE.test(id)) return null;
  return { entityType, id };
}

/**
 * Validate a target ref for a link kind, before hitting the server.
 *
 * @param kind The selected edge kind.
 * @param toRef The operator's raw input (trimmed by the caller or here).
 * @returns `null` when acceptable, else why it was rejected.
 */
export function validateToRef(
  kind: PersonLinkKind,
  toRef: string,
): ToRefProblem | null {
  const trimmed = toRef.trim();
  if (trimmed.length === 0) return "required";
  const parsed = parseEntityRef(trimmed);
  if (parsed === null) return "malformed";
  // An unknown entity type lands here too: from this form's point of
  // view "not the type this kind wants" is the actionable statement.
  if (parsed.entityType !== expectedTargetType(kind)) return "wrong-type";
  return null;
}
