import type { ApiErrorBody, ApiResponse } from "./types.js";

/** Construction options for {@link ApiClient}. */
export interface ClientOptions {
  /** Base URL of the API; trailing slashes are stripped. */
  baseUrl: string;
  /**
   * Fetch implementation to use. Inject `event.fetch` in SvelteKit load
   * functions, or a stub in tests; defaults to the global `fetch`.
   */
  fetch?: typeof fetch;
  /** Extra headers merged onto every request (e.g. auth tokens). */
  headers?: Record<string, string>;
}

/** Per-request options accepted by the {@link ApiClient} verb methods. */
export interface RequestOptions {
  /** Query-string params; `undefined`/`null` values are skipped. */
  query?: Record<string, string | number | boolean | undefined | null>;
  /** Request body; JSON-serialized when present. */
  body?: unknown;
  /** Per-request headers, merged over (and overriding) the defaults. */
  headers?: Record<string, string>;
  /** Abort signal for cancelling the in-flight request. */
  signal?: AbortSignal;
}

/**
 * Error thrown for any non-success HTTP outcome (network-level failures
 * still surface as the underlying fetch rejection).
 *
 * Carries the HTTP status plus the service's structured error body so
 * callers can branch on {@link ApiError.isNotFound} / `isConflict` /
 * `isValidation` rather than re-inspecting raw status codes.
 */
export class ApiError extends Error {
  /** HTTP status code of the failed response. */
  readonly status: number;
  /** Machine-readable error code from the body, or `"UNKNOWN"`. */
  readonly code: string;
  /** Optional structured error details from the body. */
  readonly details: unknown;

  /**
   * @param status - HTTP status code of the response.
   * @param body - Parsed service error body, or `null` if none/unparseable.
   * @param fallbackMessage - Used as the message when the body has none.
   */
  constructor(
    status: number,
    body: ApiErrorBody | null,
    fallbackMessage?: string,
  ) {
    // Prefer the server-supplied message; degrade to a generic "HTTP <status>".
    super(body?.message ?? fallbackMessage ?? `HTTP ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.code = body?.code ?? "UNKNOWN";
    this.details = body?.details;
  }

  /** True for `404 Not Found` (e.g. unknown place id). */
  get isNotFound(): boolean {
    return this.status === 404;
  }
  /** True for `409 Conflict` (e.g. duplicate detected on create). */
  get isConflict(): boolean {
    return this.status === 409;
  }
  /** True for `422 Unprocessable Entity` (validation failure). */
  get isValidation(): boolean {
    return this.status === 422;
  }
}

// Tiny fetch wrapper that understands the Place Service ApiResponse<T>
// envelope. Allows fetch injection for SSR load functions (use
// `event.fetch`) and tests.
export class ApiClient {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;
  private readonly defaultHeaders: Record<string, string>;

  /** @param options - Base URL, optional fetch impl, and default headers. */
  constructor(options: ClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/+$/, ""); // normalize: no trailing slash
    // Bind global fetch so it isn't called with the wrong `this`.
    this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
    this.defaultHeaders = {
      "content-type": "application/json",
      accept: "application/json",
      ...options.headers,
    };
  }

  /**
   * Issue a `GET` request and return the unwrapped `data` payload.
   * @typeParam T - Expected response payload type.
   */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /**
   * Issue a `POST` request and return the unwrapped `data` payload.
   * @typeParam T - Expected response payload type.
   */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }
  /**
   * Issue a `PUT` request and return the unwrapped `data` payload.
   * @typeParam T - Expected response payload type.
   */
  put<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PUT", path, opts);
  }
  /**
   * Issue a `DELETE` request; defaults to `void` for no-content responses.
   * @typeParam T - Expected response payload type.
   */
  delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("DELETE", path, opts);
  }

  /**
   * Core request pipeline: build the URL, send, then unwrap the
   * `ApiResponse<T>` envelope into either a value or an {@link ApiError}.
   *
   * @typeParam T - Expected response payload type.
   * @param method - HTTP method verb.
   * @param path - Request path (absolute or relative to the base URL).
   * @param opts - Optional query/body/headers/signal.
   * @returns The envelope's `data` field (or `undefined` for 204/empty).
   * @throws {ApiError} On non-2xx status, `success: false`, or non-JSON body.
   */
  private async request<T>(
    method: string,
    path: string,
    opts: RequestOptions = {},
  ): Promise<T> {
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

    // 204 No Content: nothing to parse (e.g. soft-delete success).
    if (response.status === 204) {
      return undefined as T;
    }

    // Read the body once as text so we can guard against empty/non-JSON.
    let parsed: ApiResponse<T> | null = null;
    const text = await response.text();
    if (text.length > 0) {
      try {
        parsed = JSON.parse(text) as ApiResponse<T>;
      } catch {
        // Body present but not JSON: surface a truncated preview.
        throw new ApiError(
          response.status,
          null,
          `Non-JSON response: ${text.slice(0, 200)}`,
        );
      }
    }

    // HTTP-level failure: prefer the envelope's structured error.
    if (!response.ok) {
      throw new ApiError(response.status, parsed?.error ?? null);
    }
    // 2xx but the envelope flags failure (defensive against the service).
    if (parsed && parsed.success === false) {
      throw new ApiError(response.status, parsed.error);
    }
    return (parsed?.data ?? undefined) as T;
  }

  /**
   * Resolve `path` against the base URL and append non-empty query params.
   * @returns The fully-qualified request URL as a string.
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
        if (v === undefined || v === null) continue; // omit absent params
        url.searchParams.set(k, String(v));
      }
    }
    return url.toString();
  }
}
