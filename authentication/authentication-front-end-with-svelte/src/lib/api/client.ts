// Lean fetch wrapper for the Authentication Service.
//
// The auth service is a loco.rs app: handlers return RAW JSON (no
// {success,data,error} envelope), so this client returns the parsed
// body directly and throws ApiError on non-2xx. A bearer token can be
// supplied per request for the protected /me and /signout endpoints.

/** Construction options for {@link ApiClient}. */
export interface ClientOptions {
  /** Base URL of the auth service (trailing slashes are trimmed). */
  baseUrl: string;
  /**
   * `fetch` implementation to use. Defaults to the global `fetch`;
   * injectable so tests and SvelteKit `load` can supply their own.
   */
  fetch?: typeof fetch;
}

/** Per-request options for {@link ApiClient.get} / {@link ApiClient.post}. */
export interface RequestOptions {
  /** Request payload; JSON-serialised. Omit to send no body. */
  body?: unknown;
  /** Bearer token for protected endpoints (`/me`, `/signout`). */
  token?: string | null;
  /** Extra request headers (merged over the JSON defaults). */
  headers?: Record<string, string>;
  /** Abort signal for cancellation. */
  signal?: AbortSignal;
}

/**
 * Error thrown by {@link ApiClient} for any non-2xx HTTP response.
 *
 * Carries the HTTP `status` and the parsed (or raw) response `body` so
 * callers can branch on the failure (see {@link isUnauthorized} /
 * {@link isBadRequest}).
 */
export class ApiError extends Error {
  /** HTTP status code of the failed response. */
  readonly status: number;
  /** Parsed JSON body, or raw text/undefined when not JSON. */
  readonly body: unknown;

  /**
   * @param status - HTTP status code.
   * @param message - Human-readable message (server message or fallback).
   * @param body - Parsed response body, if any.
   */
  constructor(status: number, message: string, body?: unknown) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.body = body;
  }

  /** True when the failure was a 401 (token missing/expired/revoked). */
  get isUnauthorized(): boolean {
    return this.status === 401;
  }
  /** True when the failure was a 400 (malformed request / validation). */
  get isBadRequest(): boolean {
    return this.status === 400;
  }
}

/**
 * Lean JSON `fetch` wrapper for the Authentication Service.
 *
 * The service is a loco.rs app returning RAW JSON (no `{success,data,error}`
 * envelope), so successful bodies are returned directly and non-2xx
 * responses throw {@link ApiError}. A bearer token may be attached per
 * request for the protected `/me` and `/signout` endpoints.
 */
export class ApiClient {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;

  /** @param options - Base URL and optional `fetch` override. */
  constructor(options: ClientOptions) {
    // Normalise away trailing slashes so URL joining is predictable.
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    // Bind to avoid "Illegal invocation" when calling a detached fetch.
    this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
  }

  /**
   * Issue a GET request and resolve the parsed JSON body.
   *
   * @typeParam T - Expected response body type.
   * @param path - Request path (with or without a leading slash).
   * @param opts - Optional bearer token, headers, signal.
   * @returns The parsed response body.
   * @throws {ApiError} On any non-2xx response.
   */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /**
   * Issue a POST request and resolve the parsed JSON body.
   *
   * @typeParam T - Expected response body type.
   * @param path - Request path (with or without a leading slash).
   * @param opts - Optional JSON body, bearer token, headers, signal.
   * @returns The parsed response body (or `undefined` for an empty body).
   * @throws {ApiError} On any non-2xx response.
   */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }

  // Shared request path for GET/POST: builds headers (+ optional bearer),
  // serialises the body, joins the URL, then parses/validates the result.
  private async request<T>(
    method: string,
    path: string,
    opts: RequestOptions = {},
  ): Promise<T> {
    const headers: Record<string, string> = {
      "content-type": "application/json",
      accept: "application/json",
      ...opts.headers,
    };
    // Attach the bearer credential only when a token is provided.
    if (opts.token) {
      headers.authorization = `Bearer ${opts.token}`;
    }

    const init: RequestInit = { method, headers, signal: opts.signal };
    // Only set a body when one was supplied (GETs send none).
    if (opts.body !== undefined) {
      init.body = JSON.stringify(opts.body);
    }

    // Resolve `path` as a *relative* reference against the base (its
    // leading slash is stripped): `api/persons` against `<origin>/api/proxy/`
    // resolves to `<origin>/api/proxy/api/persons`. A still-absolute path
    // (one kept starting with `/`) would instead replace the base URL's
    // entire path per the URL spec, silently discarding `/api/proxy` —
    // the bug this fixed 2026-08-03 (tasks.md FE-2).
    const url = new URL(
      path.startsWith("/") ? path.slice(1) : path,
      `${this.baseUrl}/`,
    ).toString();
    const response = await this.fetchFn(url, init);

    // Read the body as text first so we can tolerate empty bodies and
    // non-JSON error pages without JSON.parse throwing on us.
    const text = await response.text();
    let parsed: unknown = undefined;
    if (text.length > 0) {
      try {
        parsed = JSON.parse(text);
      } catch {
        // Non-JSON body on an error response: surface a truncated
        // snippet of the raw text rather than a parse failure.
        if (!response.ok) {
          throw new ApiError(response.status, text.slice(0, 200));
        }
      }
    }

    // Any non-2xx becomes an ApiError, preferring the server's message.
    if (!response.ok) {
      const message = extractMessage(parsed) ?? `HTTP ${response.status}`;
      throw new ApiError(response.status, message, parsed);
    }
    // Success: hand back the parsed body (undefined for an empty body).
    return parsed as T;
  }
}

// Pull a human-readable message out of a JSON error body, checking the
// common loco.rs/server field names in priority order. Returns undefined
// when the body is not an object or carries none of those fields.
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
