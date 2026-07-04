// Resource-bound wrapper over ApiClient for the auth endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { CurrentUser, LoginResponse } from "./types";

/**
 * Resource-bound facade over {@link ApiClient} for the five auth endpoints.
 *
 * Each method maps one-to-one onto an Authentication Service route and
 * keeps the magic-link flow (signup/request → verify → me → signout)
 * readable at the call site. All HTTP errors surface as {@link ApiError}.
 */
export class AuthRepository {
  /** @param http - Pre-configured client pointing at the auth service. */
  constructor(private readonly http: ApiClient) {}

  /**
   * Convenience constructor wiring an {@link ApiClient} at {@link API_BASE_URL}.
   *
   * @param fetchFn - Optional `fetch` override (e.g. a SvelteKit `load`
   *   fetch, or a test spy). Defaults to the global `fetch`.
   * @returns A repository ready to call the auth endpoints.
   */
  static withFetch(fetchFn?: typeof fetch): AuthRepository {
    return new AuthRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /**
   * Create a passwordless account and trigger a magic-link email
   * (`POST /api/auth/signup`).
   *
   * @param email - New account email address.
   * @param name - Optional display name; omitted (drops out of the JSON
   *   body) when not provided.
   * @param locale - Optional email-language hint; when omitted it drops
   *   out of the body and the service defaults to English.
   * @returns Resolves once the request is accepted (empty body).
   * @throws {ApiError} On a non-2xx response.
   */
  /// Create a passwordless account and trigger a magic link. The
  /// optional `locale` selects the magic-link email language; when
  /// omitted it drops out of the JSON body and the service defaults to
  /// English.
  signup(email: string, name?: string, locale?: string): Promise<unknown> {
    return this.http.post("/api/auth/signup", {
      body: { email, name, locale },
    });
  }

  /**
   * Request a magic link for an existing account — sign in
   * (`POST /api/auth/magic-link`).
   *
   * @param email - Existing account email address.
   * @param locale - Optional email-language hint (see {@link signup}).
   * @returns Resolves once the request is accepted (empty body).
   * @throws {ApiError} On a non-2xx response.
   */
  /// Request a magic link for an existing account (sign in). `locale`
  /// is optional (see `signup`).
  requestMagicLink(email: string, locale?: string): Promise<unknown> {
    return this.http.post("/api/auth/magic-link", { body: { email, locale } });
  }

  /**
   * Consume a magic-link token, returning an access token + profile
   * (`GET /api/auth/magic-link/{token}`).
   *
   * @param token - The single-use magic-link token from the email link;
   *   URL-encoded before being placed in the path.
   * @returns The {@link LoginResponse} (access token + profile).
   * @throws {ApiError} When the token is invalid or expired.
   */
  /// Consume a magic-link token, returning an access token + profile.
  verify(token: string): Promise<LoginResponse> {
    return this.http.get<LoginResponse>(
      `/api/auth/magic-link/${encodeURIComponent(token)}`,
    );
  }

  /**
   * Fetch the current user for a bearer token (`GET /api/auth/me`).
   *
   * @param token - The access token (bearer credential).
   * @returns The authenticated {@link CurrentUser}.
   * @throws {ApiError} 401 when the token is missing/expired/revoked.
   */
  /// Current user for a bearer token.
  me(token: string): Promise<CurrentUser> {
    return this.http.get<CurrentUser>("/api/auth/me", { token });
  }

  /**
   * Revoke the current session server-side (`POST /api/auth/signout`).
   *
   * @param token - The access token to revoke.
   * @returns Resolves once revocation is accepted (empty body).
   * @throws {ApiError} On a non-2xx response.
   */
  /// Revoke the current session.
  signout(token: string): Promise<unknown> {
    return this.http.post("/api/auth/signout", { token });
  }
}
