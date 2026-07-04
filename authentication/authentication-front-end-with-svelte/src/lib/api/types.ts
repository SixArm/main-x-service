// Types mirroring the Authentication Service responses.
// Source of truth: authentication-service-with-loco/src/views/auth.rs.
// These shapes must stay in lockstep with the Rust view structs; the
// field names match the raw JSON the loco.rs service emits (no envelope).

/**
 * Response body of `GET /api/auth/magic-link/{token}` — the verify step.
 *
 * Consuming a valid magic-link token returns the issued access token plus
 * the user's profile. `token` is the RS256 JWT used as the federation
 * bearer credential across Main X services.
 */
/// Returned by GET /api/auth/magic-link/{token} (the verify step).
export interface LoginResponse {
  /** RS256 JWT access token (the federation bearer credential). */
  token: string;
  /** Public/stable user id (UUID). */
  pid: string;
  /** Display name. */
  name: string;
  /** Account email address. */
  email: string;
  /** Whether the account's email is verified. */
  is_verified: boolean;
}

/**
 * Response body of `GET /api/auth/me` — the current authenticated user.
 *
 * Returned for a valid bearer token; the cached copy backs the account
 * dashboard. Deliberately omits the token (the caller already holds it).
 */
/// Returned by GET /api/auth/me.
export interface CurrentUser {
  /** Public/stable user id (UUID). */
  pid: string;
  /** Display name. */
  name: string;
  /** Account email address. */
  email: string;
}
