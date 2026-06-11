// Resource-bound wrapper over ApiClient for the auth endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { CurrentUser, LoginResponse } from "./types";

export class AuthRepository {
    constructor(private readonly http: ApiClient) {}

    static withFetch(fetchFn?: typeof fetch): AuthRepository {
        return new AuthRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    /// Create a passwordless account and trigger a magic link.
    signup(email: string, name?: string): Promise<unknown> {
        return this.http.post("/api/auth/signup", { body: { email, name } });
    }

    /// Request a magic link for an existing account (sign in).
    requestMagicLink(email: string): Promise<unknown> {
        return this.http.post("/api/auth/magic-link", { body: { email } });
    }

    /// Consume a magic-link token, returning an access token + profile.
    verify(token: string): Promise<LoginResponse> {
        return this.http.get<LoginResponse>(`/api/auth/magic-link/${encodeURIComponent(token)}`);
    }

    /// Current user for a bearer token.
    me(token: string): Promise<CurrentUser> {
        return this.http.get<CurrentUser>("/api/auth/me", { token });
    }

    /// Revoke the current session.
    signout(token: string): Promise<unknown> {
        return this.http.post("/api/auth/signout", { token });
    }
}
