// Types mirroring the Authentication Service responses.
// Source of truth: authentication-service-rust-crate/src/views/auth.rs.

/// Returned by GET /api/auth/magic-link/{token} (the verify step).
export interface LoginResponse {
    token: string;
    pid: string;
    name: string;
    email: string;
    is_verified: boolean;
}

/// Returned by GET /api/auth/me.
export interface CurrentUser {
    pid: string;
    name: string;
    email: string;
}
