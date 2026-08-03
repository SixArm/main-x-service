import type { ApiErrorBody, ApiResponse } from "./types.js";

/**
 * Construction options for {@link ApiClient}.
 *
 * @property baseUrl - Origin the client targets; trailing slashes are stripped.
 * @property fetch - Optional fetch implementation to inject (SvelteKit's
 *   `event.fetch` for SSR, or a mock in tests). Defaults to the global.
 * @property headers - Extra default headers merged onto every request.
 */
export interface ClientOptions {
  baseUrl: string;
  fetch?: typeof fetch;
  headers?: Record<string, string>;
}

/**
 * Per-call options for an {@link ApiClient} request.
 *
 * @property query - Query-string params; `undefined`/`null` values are skipped.
 * @property body - JSON-serialised into the request body when present.
 * @property headers - Per-request header overrides merged over the defaults.
 * @property signal - `AbortSignal` to cancel the request.
 */
export interface RequestOptions {
  query?: Record<string, string | number | boolean | undefined | null>;
  body?: unknown;
  headers?: Record<string, string>;
  signal?: AbortSignal;
}

/**
 * Typed error thrown for any non-success response (non-2xx status, or a
 * 2xx envelope with `success: false`). Carries the HTTP `status`, the
 * machine `code` from the error envelope, and any endpoint-specific
 * `details` (e.g. duplicate `MatchResult[]` on a 409). Callers branch
 * on the `isNotFound` / `isConflict` / `isValidation` getters.
 */
export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly details: unknown;

  /**
   * @param status - HTTP status code of the failed response.
   * @param body - Parsed error envelope, or `null` if none was available.
   * @param fallbackMessage - Message to use when the envelope has none.
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

  /** True when the resource was not found (HTTP 404). */
  get isNotFound(): boolean {
    return this.status === 404;
  }
  /** True on a duplicate/conflict (HTTP 409); `details` holds candidate matches. */
  get isConflict(): boolean {
    return this.status === 409;
  }
  /** True when the body failed server-side validation (HTTP 422). */
  get isValidation(): boolean {
    return this.status === 422;
  }
}

/**
 * Tiny fetch wrapper that understands the Course Service
 * {@link ApiResponse} envelope: it unwraps `data` on success and
 * raises {@link ApiError} on failure. Allows fetch injection for SSR
 * load functions (pass `event.fetch`) and tests. Stateless aside from
 * its base URL and default headers, so one instance can be shared
 * freely.
 */
export class ApiClient {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;
  private readonly defaultHeaders: Record<string, string>;

  /** @param options - Base URL, optional fetch, and default headers — see {@link ClientOptions}. */
  constructor(options: ClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
    this.defaultHeaders = {
      "content-type": "application/json",
      accept: "application/json",
      ...options.headers,
    };
  }

  /**
   * Issue a GET and return the unwrapped payload.
   * @typeParam T - Expected payload type.
   * @throws {ApiError} On any non-success response.
   */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /**
   * Issue a POST (JSON body via `opts.body`) and return the payload.
   * @typeParam T - Expected payload type.
   * @throws {ApiError} On any non-success response.
   */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }
  /**
   * Issue a PUT (JSON body via `opts.body`) and return the payload.
   * @typeParam T - Expected payload type.
   * @throws {ApiError} On any non-success response.
   */
  put<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PUT", path, opts);
  }
  /**
   * Issue a DELETE; resolves to `undefined` for a 204 No Content.
   * @typeParam T - Expected payload type (defaults to `void`).
   * @throws {ApiError} On any non-success response.
   */
  delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("DELETE", path, opts);
  }

  /**
   * Core request pipeline shared by all verbs: builds the URL,
   * serialises the body, awaits the response, then unwraps the
   * {@link ApiResponse} envelope or throws {@link ApiError}.
   *
   * @throws {ApiError} On a 204-less non-JSON body, a non-2xx status,
   *   or a 2xx envelope whose `success` is `false`.
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

    // 204 No Content (e.g. DELETE) carries no envelope to unwrap.
    if (response.status === 204) {
      return undefined as T;
    }

    // Read the body as text first so we can give a useful error on
    // non-JSON responses instead of an opaque SyntaxError. Empty
    // bodies stay `null` and fall through to the data check below.
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

    // Non-2xx → error, surfacing the envelope's error body if present.
    if (!response.ok) {
      throw new ApiError(response.status, parsed?.error ?? null);
    }
    // A 2xx can still signal failure via `success: false`.
    if (parsed && parsed.success === false) {
      throw new ApiError(response.status, parsed.error);
    }
    return (parsed?.data ?? undefined) as T;
  }

  /**
   * Join a request path onto the base URL and append query params.
   * Leading-slash-normalises `path`, and uses the trailing slash on
   * the base so relative paths resolve against the origin root.
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
        // Skip absent params so they don't serialise as the
        // string "undefined"/"null" in the query string.
        if (v === undefined || v === null) continue;
        url.searchParams.set(k, String(v));
      }
    }
    return url.toString();
  }
}
