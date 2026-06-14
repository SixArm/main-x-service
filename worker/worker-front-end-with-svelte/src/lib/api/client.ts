import type { ApiErrorBody, ApiResponse } from "./types.js";

/** Construction options for {@link ApiClient}. */
export interface ClientOptions {
    /** Base URL of the API; trailing slashes are stripped. */
    baseUrl: string;
    /**
     * `fetch` implementation to use. Inject `event.fetch` in SvelteKit load
     * functions, or a mock in tests. Defaults to the global `fetch`.
     */
    fetch?: typeof fetch;
    /** Extra default headers merged into every request. */
    headers?: Record<string, string>;
}

/** Per-request options for {@link ApiClient} verb methods. */
export interface RequestOptions {
    /** Query-string params; `undefined`/`null` values are skipped. */
    query?: Record<string, string | number | boolean | undefined | null>;
    /** Request body; JSON-stringified when present. */
    body?: unknown;
    /** Per-request headers, merged over the client defaults. */
    headers?: Record<string, string>;
    /** Abort signal to cancel the request. */
    signal?: AbortSignal;
}

/**
 * Error thrown by {@link ApiClient} for any non-2xx response or
 * `success: false` envelope. Carries the HTTP status and the service's
 * machine-readable error code/details so callers can branch on them.
 */
export class ApiError extends Error {
    /** HTTP status code of the failed response. */
    readonly status: number;
    /** Service error code (`UNKNOWN` if none provided). */
    readonly code: string;
    /** Endpoint-specific extra detail (e.g. duplicate candidates on 409). */
    readonly details: unknown;

    /**
     * @param status - HTTP status of the response.
     * @param body - Parsed error envelope, or null if none was available.
     * @param fallbackMessage - Message to use when the body has none.
     */
    constructor(status: number, body: ApiErrorBody | null, fallbackMessage?: string) {
        super(body?.message ?? fallbackMessage ?? `HTTP ${status}`);
        this.name = "ApiError";
        this.status = status;
        this.code = body?.code ?? "UNKNOWN";
        this.details = body?.details;
    }

    /** True for HTTP 404 — record not found. */
    get isNotFound(): boolean { return this.status === 404; }
    /** True for HTTP 409 — conflict, used for duplicate detection on create. */
    get isConflict(): boolean { return this.status === 409; }
    /** True for HTTP 422 — request failed server-side validation. */
    get isValidation(): boolean { return this.status === 422; }
}

/**
 * Tiny `fetch` wrapper that understands the Worker Service
 * {@link ApiResponse} envelope: it unwraps `data` on success and throws an
 * {@link ApiError} otherwise. Allows `fetch` injection for SvelteKit SSR
 * load functions (pass `event.fetch`) and for unit tests. Stateless aside
 * from base URL + default headers, so one instance can be freely shared.
 */
export class ApiClient {
    /** Base URL with trailing slashes stripped. */
    private readonly baseUrl: string;
    /** The (possibly injected) fetch implementation. */
    private readonly fetchFn: typeof fetch;
    /** Headers sent on every request unless overridden per-request. */
    private readonly defaultHeaders: Record<string, string>;

    /** @param options - Base URL, optional fetch, and default headers. */
    constructor(options: ClientOptions) {
        // Strip trailing slashes so buildUrl can append paths predictably.
        this.baseUrl = options.baseUrl.replace(/\/+$/, "");
        // Bind to globalThis so the default fetch keeps its correct `this`.
        this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
        this.defaultHeaders = {
            "content-type": "application/json",
            "accept": "application/json",
            ...options.headers,
        };
    }

    /**
     * Issue a GET request and return the unwrapped payload.
     * @returns The `data` field of the response envelope.
     * @throws {ApiError} On non-2xx or `success: false`.
     */
    get<T>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("GET", path, opts);
    }
    /**
     * Issue a POST request and return the unwrapped payload.
     * @returns The `data` field of the response envelope.
     * @throws {ApiError} On non-2xx or `success: false`.
     */
    post<T>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("POST", path, opts);
    }
    /**
     * Issue a PUT request and return the unwrapped payload.
     * @returns The `data` field of the response envelope.
     * @throws {ApiError} On non-2xx or `success: false`.
     */
    put<T>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("PUT", path, opts);
    }
    /**
     * Issue a DELETE request. Defaults to `void` since the service replies
     * `204 No Content`.
     * @throws {ApiError} On non-2xx or `success: false`.
     */
    delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("DELETE", path, opts);
    }

    /**
     * Shared request pipeline: build URL, send, then unwrap the envelope or
     * throw {@link ApiError}.
     * @throws {ApiError} On non-2xx, `success: false`, or non-JSON bodies.
     */
    private async request<T>(method: string, path: string, opts: RequestOptions = {}): Promise<T> {
        const url = this.buildUrl(path, opts.query);
        const init: RequestInit = {
            method,
            headers: { ...this.defaultHeaders, ...opts.headers },
            signal: opts.signal,
        };
        if (opts.body !== undefined) {
            init.body = JSON.stringify(opts.body);
        }

        const response = await this.fetchFn(url, init);

        // 204 No Content (e.g. DELETE) has no body to parse.
        if (response.status === 204) {
            return undefined as T;
        }

        // Read as text first so we can surface non-JSON bodies in the error.
        let parsed: ApiResponse<T> | null = null;
        const text = await response.text();
        if (text.length > 0) {
            try {
                parsed = JSON.parse(text) as ApiResponse<T>;
            } catch {
                // Body wasn't JSON — likely a proxy/gateway error page.
                throw new ApiError(response.status, null, `Non-JSON response: ${text.slice(0, 200)}`);
            }
        }

        // HTTP-level failure: prefer the parsed envelope's error if present.
        if (!response.ok) {
            throw new ApiError(response.status, parsed?.error ?? null);
        }
        // Application-level failure signalled inside a 2xx envelope.
        if (parsed && parsed.success === false) {
            throw new ApiError(response.status, parsed.error);
        }
        return (parsed?.data ?? undefined) as T;
    }

    /** Join the path onto the base URL and append non-nullish query params. */
    private buildUrl(path: string, query?: RequestOptions["query"]): string {
        // Trailing slash on base + leading slash on path keeps URL resolution
        // anchored at the origin rather than relative to the last path segment.
        const url = new URL(path.startsWith("/") ? path : `/${path}`, `${this.baseUrl}/`);
        if (query) {
            for (const [k, v] of Object.entries(query)) {
                // Skip nullish values so they aren't serialized as "undefined".
                if (v === undefined || v === null) continue;
                url.searchParams.set(k, String(v));
            }
        }
        return url.toString();
    }
}
