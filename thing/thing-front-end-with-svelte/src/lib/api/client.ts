import type { ApiErrorBody, ApiResponse } from "./types.js";

/**
 * Construction options for {@link ApiClient}.
 *
 * `fetch` is injectable so SSR load functions can pass SvelteKit's
 * `event.fetch` (for cookie/relative-URL handling) and tests can supply a
 * mock; it defaults to the global `fetch`. `headers` are merged into the
 * defaults on every request.
 */
export interface ClientOptions {
  baseUrl: string;
  fetch?: typeof fetch;
  headers?: Record<string, string>;
}

/**
 * Per-request options. `query` values that are `undefined`/`null` are
 * dropped from the query string; `body` is JSON-serialised; `signal`
 * allows cancellation via an `AbortController`.
 */
export interface RequestOptions {
  query?: Record<string, string | number | boolean | undefined | null>;
  body?: unknown;
  headers?: Record<string, string>;
  signal?: AbortSignal;
}

/**
 * Error thrown for any non-success HTTP response or failed
 * {@link ApiResponse} envelope.
 *
 * Carries the HTTP `status`, the service's stable error `code`, and the
 * opaque `details` payload, so callers can branch on the kind of failure
 * (see the `isNotFound` / `isConflict` / `isValidation` accessors) without
 * string-matching messages.
 */
export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly details: unknown;

  /**
   * @param status - HTTP status code of the failed response.
   * @param body - Parsed error envelope, or `null` if none was available.
   * @param fallbackMessage - Used as the message when `body.message` is absent.
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

  /** True for HTTP 404 — the requested record does not exist. */
  get isNotFound(): boolean {
    return this.status === 404;
  }
  /** True for HTTP 409 — duplicate detected; `details` holds the candidates. */
  get isConflict(): boolean {
    return this.status === 409;
  }
  /** True for HTTP 422 — request failed server-side validation. */
  get isValidation(): boolean {
    return this.status === 422;
  }
}

/**
 * Thin `fetch` wrapper that speaks the Thing Service's
 * {@link ApiResponse} envelope: it sets JSON headers, builds URLs, and on
 * every call unwraps `data` or throws {@link ApiError}, so callers work
 * with plain typed payloads.
 *
 * Stateless aside from base URL + headers, so one instance can be shared
 * or reconstructed per page freely.
 */
// Tiny fetch wrapper that understands the Thing Service ApiResponse<T>
// envelope. Allows fetch injection for SSR load functions (use
// `event.fetch`) and tests.
export class ApiClient {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;
  private readonly defaultHeaders: Record<string, string>;

  /** @param options - Base URL, optional `fetch` override, and default headers. */
  constructor(options: ClientOptions) {
    // Strip trailing slashes so URL joining in buildUrl() is predictable.
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    // Bind to globalThis so the global fetch keeps its correct receiver.
    this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
    this.defaultHeaders = {
      "content-type": "application/json",
      accept: "application/json",
      ...options.headers,
    };
  }

  /**
   * Issue a `GET` and return the unwrapped `data`.
   * @typeParam T - Expected payload type.
   * @returns The envelope's `data`.
   * @throws {ApiError} On non-2xx response or `success: false` envelope.
   */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /**
   * Issue a `POST` (typically with `opts.body`) and return the data.
   * @throws {ApiError} On non-2xx response or `success: false` envelope.
   */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }
  /**
   * Issue a `PUT` (typically with `opts.body`) and return the data.
   * @throws {ApiError} On non-2xx response or `success: false` envelope.
   */
  put<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PUT", path, opts);
  }
  /**
   * Issue a `DELETE`. Defaults `T` to `void` since deletes usually return
   * 204 No Content (resolves to `undefined`).
   * @throws {ApiError} On non-2xx response or `success: false` envelope.
   */
  delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("DELETE", path, opts);
  }

  /**
   * Core request pipeline shared by all verbs: build the URL, send the
   * request, then interpret the response against the envelope contract.
   * @throws {ApiError} On non-JSON body, non-2xx status, or failed envelope.
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
    // Only attach a body when one was supplied (GET/DELETE usually omit it).
    if (opts.body !== undefined) {
      init.body = JSON.stringify(opts.body);
    }

    const response = await this.fetchFn(url, init);

    // 204 No Content has no envelope to parse — resolve to undefined.
    if (response.status === 204) {
      return undefined as T;
    }

    // Read the body as text first so we can give a useful error on
    // non-JSON responses (e.g. an HTML error page from a proxy).
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

    // Transport-level failure: surface the envelope error if we have one.
    if (!response.ok) {
      throw new ApiError(response.status, parsed?.error ?? null);
    }
    // Application-level failure on a 2xx: the envelope still says failed.
    if (parsed && parsed.success === false) {
      throw new ApiError(response.status, parsed.error);
    }
    // Success: hand back just the inner data (undefined if the body was empty).
    return (parsed?.data ?? undefined) as T;
  }

  /**
   * Resolve `path` against the base URL and append non-nullish query
   * params. Uses the `URL` constructor (with a trailing-slash base) so
   * both absolute (`/api/...`) and relative paths join correctly.
   */
  private buildUrl(path: string, query?: RequestOptions["query"]): string {
    const url = new URL(
      path.startsWith("/") ? path : `/${path}`,
      `${this.baseUrl}/`,
    );
    if (query) {
      for (const [k, v] of Object.entries(query)) {
        // Skip undefined/null so optional params don't appear as "null".
        if (v === undefined || v === null) continue;
        url.searchParams.set(k, String(v));
      }
    }
    return url.toString();
  }
}
