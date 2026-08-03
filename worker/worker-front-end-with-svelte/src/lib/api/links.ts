// Pure, dependency-free helpers for cross-service entity links.
//
// These mirror the worker service's `validate_edge`
// (`src/api/rest/links.rs`) on the client so an operator sees the reason
// inline instead of learning the rule from a 422. The service remains the
// authority — this is a UX pre-check, never a substitute for it. Kept pure
// (no fetch, no Svelte) so the accept/reject matrix is unit-testable.

import type { WorkerEdgeKind } from "./types.js";

/**
 * The edge kinds a worker may originate, in the order the picker shows
 * them. Mirrors the service's `PERMITTED_KINDS`.
 */
export const WORKER_EDGE_KINDS: readonly WorkerEdgeKind[] = [
  "same_identity",
  "employed_by",
];

/**
 * The entity type each edge kind must point at (cross-service-linking §9).
 * `same_identity` resolves one human across the person and worker
 * registries; `employed_by` is an affiliation to an organization.
 */
export const EDGE_KIND_TARGET: Record<WorkerEdgeKind, string> = {
  same_identity: "person",
  employed_by: "organization",
};

/** The entity type `kind` requires on the far end (e.g. `"person"`). */
export function targetEntityType(kind: WorkerEdgeKind): string {
  return EDGE_KIND_TARGET[kind];
}

/** A placeholder-shaped example ref for `kind`, e.g. `person:<uuid>`. */
export function targetRefExample(kind: WorkerEdgeKind): string {
  return `${targetEntityType(kind)}:<uuid>`;
}

/**
 * Why a `to_ref` was rejected client-side:
 * - `required` — the field is empty.
 * - `malformed` — not a `<type>:<uuid>` URN.
 * - `wrong_target` — well-formed, but the wrong entity type for the kind.
 */
export type ToRefProblem = "required" | "malformed" | "wrong_target";

// Canonical UUID form; the service parses `to_ref` with `Uuid::parse_str`
// after splitting on the single ':'.
const UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * Check a typed `to_ref` against the selected edge kind.
 *
 * @param kind - The edge kind the operator selected.
 * @param toRef - The raw text from the field (leading/trailing space ok).
 * @returns `null` when acceptable, else the {@link ToRefProblem}.
 */
export function checkToRef(
  kind: WorkerEdgeKind,
  toRef: string,
): ToRefProblem | null {
  const raw = toRef.trim();
  if (raw.length === 0) return "required";
  const separator = raw.indexOf(":");
  // A leading ':' (empty type) or no ':' at all is malformed, matching
  // `EntityRef::from_str`'s split_once guard.
  if (separator <= 0) return "malformed";
  const entityType = raw.slice(0, separator).toLowerCase();
  const id = raw.slice(separator + 1);
  if (!UUID_PATTERN.test(id)) return "malformed";
  if (entityType !== targetEntityType(kind)) return "wrong_target";
  return null;
}

/**
 * Check the optional confidence field. A number input binds to `null`
 * when blank, which means "not supplied" and is valid.
 *
 * @param value - The bound field value.
 * @returns `"invalid"` when present but outside `[0, 1]` (or not finite),
 *   else `null`.
 */
export function checkConfidence(value: number | null): "invalid" | null {
  if (value === null) return null;
  if (!Number.isFinite(value) || value < 0 || value > 1) return "invalid";
  return null;
}
