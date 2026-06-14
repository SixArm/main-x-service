// Lean fetch wrapper for the Case Service.
//
// The service is a loco.rs app: handlers return RAW JSON (no
// {success,data,error} envelope), so this client returns the parsed
// body directly and throws ApiError on non-2xx.
//
// Auth: by default the client reads the operator's bearer token from the
// shared session store (`$lib/auth.svelte`) on each request and attaches
// it as `Authorization: Bearer <token>`, so it travels automatically once
// the operator authenticates the SPA. A per-call `token` overrides the
// store: pass a string to force a token, or `null` to omit the header.
// The store getter is injectable for tests.

// The default bearer-token source: the shared reactive session store.
// Imported as a function so each request reads the *current* token.
import { token as sessionToken } from "$lib/auth.svelte";

/**
 * Construction-time configuration for {@link ApiClient}.
 */
export interface ClientOptions {
    /** Absolute origin (and optional prefix) of the Case Service, e.g. `http://localhost:5150`. Trailing slashes are trimmed. */
    baseUrl: string;
    /** Override the `fetch` implementation (SSR, tests, instrumentation). Defaults to the global `fetch`. */
    fetch?: typeof fetch;
    /** Override the default session-token source (testing/seam). */
    tokenSource?: () => string | null;
}

/**
 * Per-request options shared by every verb on {@link ApiClient}.
 */
export interface RequestOptions {
    /** JSON request body; serialised with `JSON.stringify`. Omit for bodyless requests (GET/DELETE). */
    body?: unknown;
    /** Per-call bearer override: a string forces that token, `null` omits the header, `undefined` falls back to the session source. */
    token?: string | null;
    /** Extra request headers, merged over (and able to override) the JSON defaults. */
    headers?: Record<string, string>;
    /** Abort signal for cancellation (e.g. component teardown). */
    signal?: AbortSignal;
}

/**
 * Error thrown for any non-2xx response. Carries the HTTP status and the
 * parsed (or raw) error body so callers can branch on them.
 */
export class ApiError extends Error {
    /** The HTTP status code of the failed response. */
    readonly status: number;
    /** The parsed JSON error body, or `undefined` when the body was empty / not JSON. */
    readonly body: unknown;

    /**
     * @param status HTTP status code of the failed response.
     * @param message Human-readable message (server-supplied where possible, else `HTTP <status>`).
     * @param body Parsed error body, if any.
     */
    constructor(status: number, message: string, body?: unknown) {
        super(message);
        this.name = "ApiError";
        this.status = status;
        this.body = body;
    }

    /** True when the status is 401 — the session token is missing/expired. */
    get isUnauthorized(): boolean {
        return this.status === 401;
    }
    /** True when the status is 400 — a malformed request. */
    get isBadRequest(): boolean {
        return this.status === 400;
    }
}

/**
 * Lean fetch wrapper for the Case Service's raw-JSON (no envelope) API.
 * Resolves to the parsed response body, or throws {@link ApiError} on any
 * non-2xx status. Holds no request state, so a single instance is reusable.
 */
export class ApiClient {
    /** Normalised base URL (trailing slashes stripped). */
    private readonly baseUrl: string;
    /** The `fetch` implementation to use for every request. */
    private readonly fetchFn: typeof fetch;
    /** Callback returning the default bearer token (session store by default). */
    private readonly tokenSource: () => string | null;

    /**
     * @param options Base URL, plus optional `fetch` and token-source seams.
     */
    constructor(options: ClientOptions) {
        this.baseUrl = options.baseUrl.replace(/\/+$/, "");
        this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
        this.tokenSource = options.tokenSource ?? sessionToken;
    }

    /**
     * Issue a GET request.
     * @typeParam T Expected shape of the parsed response body.
     * @param path Request path (absolute or leading-slash-optional).
     * @param opts Optional headers/token/signal.
     * @returns The parsed response body.
     * @throws {ApiError} on any non-2xx response.
     */
    get<T>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("GET", path, opts);
    }
    /**
     * Issue a POST request.
     * @typeParam T Expected shape of the parsed response body.
     * @param path Request path.
     * @param opts Options; usually carries `body`.
     * @returns The parsed response body.
     * @throws {ApiError} on any non-2xx response.
     */
    post<T>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("POST", path, opts);
    }
    /**
     * Issue a PUT request.
     * @typeParam T Expected shape of the parsed response body.
     * @param path Request path.
     * @param opts Options; usually carries `body`.
     * @returns The parsed response body.
     * @throws {ApiError} on any non-2xx response.
     */
    put<T>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("PUT", path, opts);
    }
    /**
     * Issue a DELETE request. Defaults `T` to `void` since soft-delete
     * returns an empty body.
     * @typeParam T Expected shape of the parsed response body.
     * @param path Request path.
     * @param opts Optional headers/token/signal.
     * @returns The parsed response body (typically `undefined`).
     * @throws {ApiError} on any non-2xx response.
     */
    delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("DELETE", path, opts);
    }

    /**
     * Core request pipeline shared by all verbs: assemble headers, resolve
     * the bearer token, send, then parse the body and map non-2xx to
     * {@link ApiError}.
     * @typeParam T Expected shape of the parsed response body.
     * @throws {ApiError} on any non-2xx response.
     */
    private async request<T>(method: string, path: string, opts: RequestOptions = {}): Promise<T> {
        // JSON defaults first, then spread caller overrides so they win.
        const headers: Record<string, string> = {
            "content-type": "application/json",
            accept: "application/json",
            ...opts.headers,
        };
        // A per-call `token` (string or explicit `null`) overrides the
        // session store; `undefined` falls back to the store.
        const token = opts.token !== undefined ? opts.token : this.tokenSource();
        if (token) {
            headers.authorization = `Bearer ${token}`;
        }

        const init: RequestInit = { method, headers, signal: opts.signal };
        if (opts.body !== undefined) {
            init.body = JSON.stringify(opts.body);
        }

        // Resolve relative paths against `baseUrl`; the trailing slash on the
        // base keeps `URL` from dropping any base path segment.
        const url = new URL(path.startsWith("/") ? path : `/${path}`, `${this.baseUrl}/`).toString();
        const response = await this.fetchFn(url, init);

        // Read as text first so we can handle empty bodies (soft-delete) and
        // non-JSON error pages without a hard parse failure.
        const text = await response.text();
        let parsed: unknown = undefined;
        if (text.length > 0) {
            try {
                parsed = JSON.parse(text);
            } catch {
                // Non-JSON body: only surface it as an error on a failed
                // response (truncated); a non-JSON 2xx body stays `undefined`.
                if (!response.ok) {
                    throw new ApiError(response.status, text.slice(0, 200));
                }
            }
        }

        if (!response.ok) {
            // Prefer a server-supplied message; otherwise a generic status line.
            const message = extractMessage(parsed) ?? `HTTP ${response.status}`;
            throw new ApiError(response.status, message, parsed);
        }
        return parsed as T;
    }
}

/**
 * Best-effort extraction of a human-readable message from a parsed error
 * body, trying the common loco.rs/JSON keys in order.
 * @param body Parsed response body (any shape).
 * @returns The first string found under `error`/`message`/`description`, else `undefined`.
 */
function extractMessage(body: unknown): string | undefined {
    if (body && typeof body === "object") {
        const record = body as Record<string, unknown>;
        for (const key of ["error", "message", "description"]) {
            const value = record[key];
            if (typeof value === "string") return value;
        }
    }
    return undefined;
}
