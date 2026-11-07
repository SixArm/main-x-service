import type { Problem } from "./types.js";

/**
 * Name of the browser-readable CSRF double-submit cookie. Must match
 * `CSRF_COOKIE` in `#lib/server/session.ts` — duplicated here (rather
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
 * Error thrown for any non-2xx API outcome.
 *
 * Carries the HTTP `status`, the problem's stable error `code`, and its
 * structured `errors` as `details`, so callers can branch on the kind of failure
 * (see the `isNotFound` / `isConflict` / `isValidation` accessors) without
 * string-matching messages.
 */
export class ApiError extends Error {
  /** HTTP status code of the failed response. */
  readonly status: number;
  /** Machine-readable error code from the problem, or `"unknown"`. */
  readonly code: string;
  /** The problem's structured `errors` (e.g. duplicate candidates). */
  readonly details: unknown;

  /**
   * @param status - HTTP status code of the failed response.
   * @param body - Parsed problem, or `null` if none was available.
   * @param fallbackMessage - Used as the message when the problem carries
   *   neither `detail` nor `title`.
   */
  constructor(status: number, body: Problem | null, fallbackMessage?: string) {
    super(body?.detail ?? body?.title ?? fallbackMessage ?? `HTTP ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.code = body?.code ?? "unknown";
    this.details = body?.errors;
  }

  /** True for HTTP 401 — missing/expired/rejected session. */
  get isUnauthorized(): boolean {
    return this.status === 401;
  }
  /** True for HTTP 403 — valid session, but the ABAC policy denied it. */
  get isForbidden(): boolean {
    return this.status === 403;
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
 * Thin `fetch` wrapper over the Thing Service: it sets JSON headers,
 * builds URLs, and on every call returns the bare JSON payload or throws
 * {@link ApiError} carrying the RFC 9457 {@link Problem}, so callers work
 * with plain typed payloads.
 *
 * Stateless aside from base URL + headers, so one instance can be shared
 * or reconstructed per page freely. Allows fetch injection for SSR load
 * functions (use `event.fetch`) and tests.
 */
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
   * Issue a `GET` and return the parsed body.
   * @typeParam T - Expected payload type.
   * @returns The parsed body.
   * @throws {ApiError} On non-2xx response.
   */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /**
   * As {@link get}, but also returns the raw {@link Response} so a
   * caller can read response headers — e.g. the `Link: <…>; rel="next"`
   * that says where a collection's next page is
   * (`agents/share/restful.md`, read with {@link nextLink}), which the
   * plain body a normal `get` returns has no room to carry.
   * @throws {ApiError} On non-2xx response.
   */
  getWithHeaders<T>(
    path: string,
    opts?: RequestOptions,
  ): Promise<{ data: T; response: Response }> {
    return this.requestWithResponse<T>("GET", path, opts);
  }
  /**
   * Issue a `POST` (typically with `opts.body`) and return the data.
   * @throws {ApiError} On non-2xx response.
   */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }
  /**
   * Issue a `PUT` (typically with `opts.body`) and return the data.
   * @throws {ApiError} On non-2xx response.
   */
  put<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PUT", path, opts);
  }
  /**
   * Issue a `DELETE`. Defaults `T` to `void` since deletes usually return
   * 204 No Content (resolves to `undefined`).
   * @throws {ApiError} On non-2xx response.
   */
  delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("DELETE", path, opts);
  }

  /**
   * Core request pipeline shared by all verbs: delegates to
   * {@link requestWithResponse} and discards the raw `Response`, which
   * every verb but {@link getWithHeaders} has no use for.
   * @throws {ApiError} On non-JSON body or non-2xx status.
   */
  private async request<T>(
    method: string,
    path: string,
    opts: RequestOptions = {},
  ): Promise<T> {
    const { data } = await this.requestWithResponse<T>(method, path, opts);
    return data;
  }

  /**
   * As {@link request}, but resolves to both the parsed body (as `data`)
   * and the raw {@link Response} — the one place headers are reachable.
   * @throws {ApiError} On non-JSON body or non-2xx status.
   */
  private async requestWithResponse<T>(
    method: string,
    path: string,
    opts: RequestOptions = {},
  ): Promise<{ data: T; response: Response }> {
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
    // Only attach a body when one was supplied (GET/DELETE usually omit it).
    if (opts.body !== undefined) {
      init.body = JSON.stringify(opts.body);
    }

    const response = await this.fetchFn(url, init);

    // 204 No Content has no body to parse — resolve to undefined.
    if (response.status === 204) {
      return { data: undefined as T, response };
    }

    // Read the body as text first so we can give a useful error on
    // non-JSON responses (e.g. an HTML error page from a proxy).
    let parsed: unknown = null;
    const text = await response.text();
    if (text.length > 0) {
      try {
        parsed = JSON.parse(text) as unknown;
      } catch {
        throw new ApiError(
          response.status,
          null,
          `Non-JSON response: ${text.slice(0, 200)}`,
        );
      }
    }

    if (!response.ok) {
      throw new ApiError(response.status, asProblem(parsed));
    }
    // Success: the body is the payload (undefined if the body was empty),
    // plus the raw response, for callers that need its headers.
    return { data: (parsed ?? undefined) as T, response };
  }

  /**
   * Resolve `path` against the base URL and append non-nullish query
   * params. Uses the `URL` constructor (with a trailing-slash base) so
   * both absolute (`/api/...`) and relative paths join correctly.
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
        // Skip undefined/null so optional params don't appear as "null".
        if (v === undefined || v === null) continue;
        url.searchParams.set(k, String(v));
      }
    }
    return url.toString();
  }
}

/**
 * The URL of `rel="next"` in a `Link` header, or `null` when there is
 * none. Only a same-origin path is taken, so a header cannot send the
 * client somewhere else.
 */
export function nextLink(header: string | null): string | null {
  if (!header) return null;
  for (const part of header.split(",")) {
    const match = /^\s*<([^>]*)>\s*;\s*rel="?next"?\s*$/.exec(part);
    const url = match?.[1];
    if (url?.startsWith("/") && !url.startsWith("//")) return url;
  }
  return null;
}

/** `value` as a problem when it is an object, else `null`. */
function asProblem(value: unknown): Problem | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Problem)
    : null;
}
