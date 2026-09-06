// Client-side length/format hints for the deterministic identifier
// schemes the service check-digit-validates server-side (SEC-M5,
// `organization-service-with-loco/src/validation.rs::identifier_problem`).
//
// This is a **pure length/format hint**, not a check-digit
// re-implementation: it never recomputes LEI's ISO 7064 MOD 97-10 check
// or GLN's GS1 mod-10 check digit — only the shape (length, charset,
// prefix) a caller can eyeball before submitting. A value that passes
// this hint can still be rejected `422` by the server (e.g. a
// syntactically well-formed but check-digit-invalid LEI); the server
// stays the sole authority, this only saves an obviously-malformed
// value a round trip. TaxId / NAICS / ISIC v4 / SIC are unconstrained
// here, exactly as the server leaves them unconstrained (ORGFE-T4).

import type { IdentifierScheme } from "$lib/api/types.js";

/** Keep only the ASCII digits of `s` (drops spaces, hyphens, dots). */
function digitsOnly(s: string): string {
  return s.replace(/[^0-9]/g, "");
}

/**
 * Returns a short, human-readable hint when `value` doesn't match the
 * expected length/format for `scheme`, or `null` when it looks
 * plausible (or the scheme carries no format hint). `scheme` may be the
 * `{ Custom: string }` variant, which — like every non-deterministic
 * scheme — is never hinted.
 */
export function identifierFormatHint(
  scheme: IdentifierScheme,
  value: string,
): string | null {
  if (typeof scheme !== "string") return null;
  const trimmed = value.trim();
  if (trimmed.length === 0) return null; // blank rows are dropped on submit, not a format error

  switch (scheme) {
    case "Lei": {
      const upper = trimmed.toUpperCase();
      const ok = upper.length === 20 && /^[A-Z0-9]{20}$/.test(upper);
      return ok ? null : "Expected 20 alphanumeric characters";
    }
    case "Duns": {
      const compact = trimmed.replace(/[ -]/g, "");
      const ok =
        compact.length === digitsOnly(compact).length &&
        digitsOnly(compact).length === 9;
      return ok ? null : "Expected 9 digits";
    }
    case "Gln": {
      const compact = trimmed.replace(/[ -]/g, "");
      const ok =
        compact.length === digitsOnly(compact).length &&
        digitsOnly(compact).length === 13;
      return ok ? null : "Expected 13 digits";
    }
    case "Vat": {
      const compact = trimmed.replace(/\s/g, "");
      const ok =
        compact.length >= 4 &&
        compact.length <= 15 &&
        /^[A-Za-z]{2}[A-Za-z0-9]+$/.test(compact);
      return ok
        ? null
        : "Expected a 2-letter country prefix followed by 2–13 alphanumerics";
    }
    default:
      return null;
  }
}
