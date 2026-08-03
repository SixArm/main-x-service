// Lean fetch wrapper for the Care Pathway Service.
//
// The service is a loco.rs app: handlers return RAW JSON (no
// {success,data,error} envelope), so this client returns the parsed
// body directly and throws ApiError on non-2xx.
//
// Auth is handled by the same-origin BFF proxy (it injects a server-
// exchanged PASETO), so this client attaches no bearer of its own — only
// an explicit per-request `token` override, if ever passed. See
// `agents/share/authentication-sessions.md`.

/** Construction options for {@link ApiClient}. */
export interface ClientOptions {
  /** Service base URL; any trailing slash is trimmed at construction. */
  baseUrl: string;
  /**
   * `fetch` implementation to use. Defaults to the global `fetch`;
   * injectable so SvelteKit's `load` fetch or a test fake can be passed.
   */
  fetch?: typeof fetch;
}

/** Per-request options for the {@link ApiClient} verb methods. */
export interface RequestOptions {
  /** JSON request body; serialized with `JSON.stringify` when present. */
  body?: unknown;
  /**
   * Per-call bearer token override. A string forces that token; an
   * explicit `null`/`""` suppresses the `Authorization` header. The BFF
   * proxy injects the bearer server-side, so this is normally omitted.
   */
  token?: string | null;
  /** Extra request headers, merged over the JSON content/accept defaults. */
  headers?: Record<string, string>;
  /** Abort signal to cancel the request. */
  signal?: AbortSignal;
}

/**
 * Error thrown for any non-2xx HTTP response. Carries the status code and
 * the parsed (or raw) response body so callers can branch on them.
 */
export class ApiError extends Error {
  /** The HTTP status code of the failing response. */
  readonly status: number;
  /** The parsed response body (or raw text fragment when not JSON). */
  readonly body: unknown;

  /**
   * @param status - HTTP status code of the response.
   * @param message - Human-readable message (server-supplied or `HTTP <n>`).
   * @param body - Optional parsed/raw response body for inspection.
   */
  constructor(status: number, message: string, body?: unknown) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.body = body;
  }

  /** `true` when the response was `401 Unauthorized` (token missing/invalid). */
  get isUnauthorized(): boolean {
    return this.status === 401;
  }
  /** `true` when the response was `400 Bad Request`. */
  get isBadRequest(): boolean {
    return this.status === 400;
  }
}

/** A `limit` / `offset` window to request. */
export interface PageRequest {
  /** Page size; omitted ⇒ the service default. */
  limit?: number;
  /** Rows to skip; omitted ⇒ 0. */
  offset?: number;
}

/** One page of a collection plus the window the service applied. */
export interface Page<T> {
  /** The rows in this page. */
  items: T[];
  /** Rows matching the query overall, ignoring the window. */
  total: number;
  /** The page size the service applied (it may have clamped yours). */
  limit: number;
  /** The offset the service applied. */
  offset: number;
}


/**
 * Lean JSON `fetch` wrapper for the loco.rs care-pathway service.
 *
 * The service returns RAW JSON (no `{success,data,error}` envelope), so
 * the verb helpers resolve the parsed body directly and throw
 * {@link ApiError} on any non-2xx response. The BFF proxy injects the
 * bearer server-side, so this client sends one only when an explicit
 * per-call `token` is passed (via `RequestOptions.token`).
 */
export class ApiClient {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;

  /** @param options - Base URL and optional `fetch` implementation. */
  constructor(options: ClientOptions) {
    // Trim trailing slash so URL joining below never doubles up.
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    // Bind the global `fetch` so it is callable detached from `globalThis`.
    this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
  }

  /**
   * Issue a `GET` and resolve the parsed JSON body as `T`.
   * @param path - Path (absolute or root-relative) under the base URL.
   * @param opts - Optional per-request options.
   */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /**
   * Issue a `POST` (typically with `opts.body`) and resolve the parsed body.
   * @param path - Path under the base URL.
   * @param opts - Optional per-request options (usually `{ body }`).
   */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }
  /**
   * Issue a `PUT` (typically with `opts.body`) and resolve the parsed body.
   * @param path - Path under the base URL.
   * @param opts - Optional per-request options (usually `{ body }`).
   */
  put<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PUT", path, opts);
  }
  /**
   * Issue a `DELETE`. The service's soft-delete returns an empty body, so
   * the default `T` is `void`.
   * @param path - Path under the base URL.
   * @param opts - Optional per-request options.
   */
  delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("DELETE", path, opts);
  }

  /**
   * Core request pipeline: build headers (+ optional bearer), send the
   * request, parse the body, and throw {@link ApiError} on non-2xx.
   *
   * @param method - HTTP verb.
   * @param path - Path under the base URL.
   * @param opts - Per-request options.
   * @returns The parsed JSON body as `T` (or `undefined` for an empty body).
   * @throws {ApiError} When the response status is not 2xx.
   */
  /**
   * `GET` one page of a collection, returning the body **and** the
   * pagination headers the service sends (`agents/share/restful.md`).
   *
   * The plain {@link get} discards response headers, which is fine for a
   * single record and useless for a collection: the total lives in
   * `X-Total-Count`, deliberately, so that adding it did not change the
   * array body every existing caller already parses.
   */
  async getPage<T>(
    path: string,
    page: PageRequest = {},
    opts: RequestOptions = {},
  ): Promise<Page<T>> {
    const query = new URLSearchParams();
    if (page.limit !== undefined) query.set("limit", String(page.limit));
    if (page.offset !== undefined) query.set("offset", String(page.offset));
    const suffix = query.toString();
    const { data, response } = await this.requestWithResponse<T[]>(
      "GET",
      suffix ? `${path}${path.includes("?") ? "&" : "?"}${suffix}` : path,
      opts,
    );
    const header = (name: string): number | undefined => {
      const raw = response.headers.get(name);
      if (raw === null) return undefined;
      const value = Number(raw);
      return Number.isFinite(value) ? value : undefined;
    };
    const items = data ?? [];
    return {
      items,
      // A service that predates the headers still works: the page length
      // is the honest fallback.
      total: header("x-total-count") ?? items.length,
      limit: header("x-limit") ?? page.limit ?? items.length,
      offset: header("x-offset") ?? page.offset ?? 0,
    };
  }

  private async request<T>(
    method: string,
    path: string,
    opts: RequestOptions = {},
  ): Promise<T> {
    const { data } = await this.requestWithResponse<T>(method, path, opts);
    return data;
  }

  /**
   * As {@link request}, but also returns the raw {@link Response} so a
   * caller can read headers (see {@link getPage}).
   */
  private async requestWithResponse<T>(
    method: string,
    path: string,
    opts: RequestOptions = {},
  ): Promise<{ data: T; response: Response }> {
    const headers: Record<string, string> = {
      "content-type": "application/json",
      accept: "application/json",
      ...opts.headers,
    };
    // The BFF proxy injects the bearer server-side; this client sends a
    // bearer only when one is explicitly passed for this request.
    if (opts.token) {
      headers.authorization = `Bearer ${opts.token}`;
    }

    const init: RequestInit = { method, headers, signal: opts.signal };
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

    // Read the body as text first so an empty body (soft-delete) and a
    // non-JSON error body are both handled without throwing on parse.
    const text = await response.text();
    let parsed: unknown = undefined;
    if (text.length > 0) {
      try {
        parsed = JSON.parse(text);
      } catch {
        // Non-JSON body: on an error response, surface a truncated
        // snippet; on success, leave `parsed` undefined.
        if (!response.ok) {
          throw new ApiError(response.status, text.slice(0, 200));
        }
      }
    }

    if (!response.ok) {
      // Prefer a server-supplied message; fall back to the status line.
      const message = extractMessage(parsed) ?? `HTTP ${response.status}`;
      throw new ApiError(response.status, message, parsed);
    }
    return { data: parsed as T, response };
  }
}

/**
 * Pull a human-readable message out of a JSON error body, checking the
 * common `error` / `message` / `description` keys in order.
 *
 * @param body - The parsed response body.
 * @returns The first string-valued message found, or `undefined`.
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
