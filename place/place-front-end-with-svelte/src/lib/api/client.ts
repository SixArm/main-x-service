import type { Problem } from "./types.js";

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
 * Error thrown for any non-2xx HTTP outcome (network-level failures
 * still surface as the underlying fetch rejection).
 *
 * Carries the HTTP status plus the parsed RFC 9457 {@link Problem} so
 * callers can branch on {@link ApiError.isNotFound} / `isConflict` /
 * `isValidation` rather than re-inspecting raw status codes.
 */
export class ApiError extends Error {
  /** HTTP status code of the failed response. */
  readonly status: number;
  /** Machine-readable error code from the problem, or `"unknown"`. */
  readonly code: string;
  /** The problem's structured `errors` (e.g. duplicate candidates). */
  readonly details: unknown;

  /**
   * @param status - HTTP status code of the response.
   * @param body - Parsed problem, or `null` if none/unparseable.
   * @param fallbackMessage - Used as the message when the body has none.
   */
  constructor(status: number, body: Problem | null, fallbackMessage?: string) {
    // Prefer the server-supplied detail, then the status title; degrade to
    // a generic "HTTP <status>".
    super(body?.detail ?? body?.title ?? fallbackMessage ?? `HTTP ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.code = body?.code ?? "unknown";
    this.details = body?.errors;
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

// Tiny fetch wrapper over the Place Service: a success is the bare JSON
// payload, and a failure is an RFC 9457 problem. Allows fetch injection for SSR load functions (use
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
   * Issue a `GET` request and return the parsed payload.
   * @typeParam T - Expected response payload type.
   */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /**
   * Issue a `POST` request and return the parsed payload.
   * @typeParam T - Expected response payload type.
   */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }
  /**
   * Issue a `PUT` request and return the parsed payload.
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
   * Core request pipeline: build the URL, send, then classify the
   * outcome as either the parsed body or an {@link ApiError}.
   *
   * @typeParam T - Expected response payload type.
   * @param method - HTTP method verb.
   * @param path - Request path (absolute or relative to the base URL).
   * @param opts - Optional query/body/headers/signal.
   * @returns The parsed body as `T` (or `undefined` for 204/empty).
   * @throws {ApiError} On non-2xx status, or a non-JSON body.
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
    let parsed: unknown = null;
    const text = await response.text();
    if (text.length > 0) {
      try {
        parsed = JSON.parse(text) as unknown;
      } catch {
        // Body present but not JSON: surface a truncated preview.
        throw new ApiError(
          response.status,
          null,
          `Non-JSON response: ${text.slice(0, 200)}`,
        );
      }
    }

    // HTTP-level failure: the body is a problem.
    if (!response.ok) {
      throw new ApiError(response.status, asProblem(parsed));
    }
    // Success: the body is the payload (undefined if none).
    return (parsed ?? undefined) as T;
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

/** `value` as a problem when it is an object, else `null`. */
function asProblem(value: unknown): Problem | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Problem)
    : null;
}
