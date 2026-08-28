import type { ApiErrorBody, ApiResponse } from "./types.js";

/**
 * Name of the browser-readable CSRF double-submit cookie. Must match
 * `CSRF_COOKIE` in `$lib/server/session.ts` — duplicated here (rather
 * than imported) because that module is server-only and SvelteKit
 * refuses to bundle it into browser code.
 */
const CSRF_COOKIE_NAME = "__Host-mxi_csrf";

/**
 * Read `CSRF_COOKIE_NAME`'s value out of `document.cookie`, or `null`
 * when absent or not running in a browser (e.g. an SSR load function
 * that injected `event.fetch` — see {@link ClientOptions.fetch}).
 */
function readCsrfCookie(): string | null {
  if (typeof document === "undefined") return null;
  const prefix = `${CSRF_COOKIE_NAME}=`;
  for (const part of document.cookie.split(";")) {
    const trimmed = part.trim();
    if (trimmed.startsWith(prefix)) {
      return decodeURIComponent(trimmed.slice(prefix.length));
    }
  }
  return null;
}

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
  constructor(
    status: number,
    body: ApiErrorBody | null,
    fallbackMessage?: string,
  ) {
    super(body?.message ?? fallbackMessage ?? `HTTP ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.code = body?.code ?? "UNKNOWN";
    this.details = body?.details;
  }

  /** True for HTTP 404 — record not found. */
  get isNotFound(): boolean {
    return this.status === 404;
  }
  /** True for HTTP 409 — conflict, used for duplicate detection on create. */
  get isConflict(): boolean {
    return this.status === 409;
  }
  /** True for HTTP 422 — request failed server-side validation. */
  get isValidation(): boolean {
    return this.status === 422;
  }
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
      accept: "application/json",
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
  private async request<T>(
    method: string,
    path: string,
    opts: RequestOptions = {},
  ): Promise<T> {
    const url = this.buildUrl(path, opts.query);
    const headers: Record<string, string> = {
      ...this.defaultHeaders,
      ...opts.headers,
    };
    // CSRF double-submit (agents/share/authentication-sessions.md §4):
    // every mutating browser-issued request echoes the readable
    // `__Host-mxi_csrf` cookie in a header, which the BFF proxy verifies
    // against the same cookie before forwarding upstream. GET/HEAD are
    // safe methods and carry no such header. `readCsrfCookie` returns
    // `null` server-side (no `document`), so an SSR load's mutating call
    // — there are none today, but if one appears — simply omits the
    // header rather than throwing.
    if (method !== "GET" && method !== "HEAD") {
      const csrfToken = readCsrfCookie();
      if (csrfToken) headers["x-csrf-token"] = csrfToken;
    }
    const init: RequestInit = { method, headers, signal: opts.signal };
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
        throw new ApiError(
          response.status,
          null,
          `Non-JSON response: ${text.slice(0, 200)}`,
        );
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
    // Resolve `path` as a *relative* reference against the base (its
    // leading slash is stripped): `api/persons` against `<origin>/api/proxy/`
    // resolves to `<origin>/api/proxy/api/persons`. A still-absolute path
    // (one kept starting with `/`) would instead replace the base URL's
    // entire path per the URL spec, silently discarding `/api/proxy` —
    // the bug this fixed 2026-08-03 (tasks.md FE-2).
    const url = new URL(
      path.startsWith("/") ? path.slice(1) : path,
      `${this.baseUrl}/`,
    );
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
