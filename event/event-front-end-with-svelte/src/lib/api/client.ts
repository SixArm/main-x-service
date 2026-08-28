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
  /** Absolute base URL the client resolves request paths against. */
  baseUrl: string;
  /** Optional fetch implementation; inject `event.fetch` under SSR or a stub in tests. */
  fetch?: typeof fetch;
  /** Extra default headers merged into every request. */
  headers?: Record<string, string>;
}

/** Per-request options for {@link ApiClient} verb methods. */
export interface RequestOptions {
  /** Query-string params; `undefined`/`null` entries are skipped. */
  query?: Record<string, string | number | boolean | undefined | null>;
  /** Request body; JSON-stringified when present (omit for GET). */
  body?: unknown;
  /** Per-request headers, overriding the client defaults. */
  headers?: Record<string, string>;
  /** Abort signal to cancel the request. */
  signal?: AbortSignal;
}

/**
 * Error thrown by {@link ApiClient} for any non-success response (HTTP
 * error status or an envelope with `success: false`). Carries the HTTP
 * `status`, the service error `code`, and any `details` for the UI to
 * branch on (see the `isNotFound`/`isConflict`/`isValidation` helpers).
 */
export class ApiError extends Error {
  /** HTTP status code of the failed response. */
  readonly status: number;
  /** Machine-readable error code from the envelope (or `"UNKNOWN"`). */
  readonly code: string;
  /** Optional structured error details from the envelope. */
  readonly details: unknown;

  /**
   * @param status - HTTP status code of the response.
   * @param body - Parsed error body, or null when none was available.
   * @param fallbackMessage - Used as the message when the body has none.
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

  /** True when the failure was a 404 Not Found. */
  get isNotFound(): boolean {
    return this.status === 404;
  }
  /** True when the failure was a 409 Conflict (e.g. duplicate on create). */
  get isConflict(): boolean {
    return this.status === 409;
  }
  /** True when the failure was a 422 Unprocessable Entity (validation error). */
  get isValidation(): boolean {
    return this.status === 422;
  }
}

/**
 * Minimal fetch wrapper that speaks the Event Service `ApiResponse<T>`
 * envelope: it unwraps `data` on success and throws {@link ApiError} on
 * failure. The fetch implementation is injectable so callers can pass
 * `event.fetch` under SSR load functions or a stub in tests.
 */
// Tiny fetch wrapper that understands the Event Service ApiResponse<T>
// envelope. Allows fetch injection for SSR load functions (use
// `event.fetch`) and tests.
export class ApiClient {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;
  private readonly defaultHeaders: Record<string, string>;

  /** @param options - Base URL, optional fetch override, and default headers. */
  constructor(options: ClientOptions) {
    // Strip trailing slashes so URL joining in buildUrl is unambiguous.
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    // Bind the global fetch so `this` is correct when no override is given.
    this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
    this.defaultHeaders = {
      "content-type": "application/json",
      accept: "application/json",
      ...options.headers,
    };
  }

  /**
   * Issue a GET request and return the unwrapped payload.
   * @typeParam T - Expected payload type.
   * @returns The envelope's `data`.
   * @throws {ApiError} On any non-success response.
   */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /**
   * Issue a POST request and return the unwrapped payload.
   * @typeParam T - Expected payload type.
   * @returns The envelope's `data`.
   * @throws {ApiError} On any non-success response.
   */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }
  /**
   * Issue a PUT request and return the unwrapped payload.
   * @typeParam T - Expected payload type.
   * @returns The envelope's `data`.
   * @throws {ApiError} On any non-success response.
   */
  put<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PUT", path, opts);
  }
  /**
   * Issue a DELETE request. Defaults to a `void` payload (204 responses).
   * @typeParam T - Expected payload type (defaults to `void`).
   * @throws {ApiError} On any non-success response.
   */
  delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("DELETE", path, opts);
  }

  /**
   * Core request pipeline: build the URL, send the request, then unwrap
   * the `ApiResponse<T>` envelope into `T` or throw {@link ApiError}.
   *
   * @returns The envelope `data`, or `undefined` for empty/204 responses.
   * @throws {ApiError} On HTTP error status, non-JSON bodies, or
   *   `success: false` envelopes.
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

    // 204 No Content carries no body to unwrap (e.g. soft-delete).
    if (response.status === 204) {
      return undefined as T;
    }

    // Read once as text so we can tolerate empty bodies and report
    // non-JSON payloads with a helpful (truncated) excerpt.
    let parsed: ApiResponse<T> | null = null;
    const text = await response.text();
    if (text.length > 0) {
      try {
        parsed = JSON.parse(text) as ApiResponse<T>;
      } catch {
        throw new ApiError(
          response.status,
          null,
          `Non-JSON response: ${text.slice(0, 200)}`,
        );
      }
    }

    // Surface transport failures using the parsed error body if present.
    if (!response.ok) {
      throw new ApiError(response.status, parsed?.error ?? null);
    }
    // Application-level failure on an otherwise-2xx response.
    if (parsed && parsed.success === false) {
      throw new ApiError(response.status, parsed.error);
    }
    return (parsed?.data ?? undefined) as T;
  }

  /**
   * Resolve `path` against the base URL and append query params,
   * skipping `undefined`/`null` values. Leading-slash-normalizes `path`
   * so relative and absolute forms behave identically.
   */
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
        if (v === undefined || v === null) continue;
        url.searchParams.set(k, String(v));
      }
    }
    return url.toString();
  }
}
