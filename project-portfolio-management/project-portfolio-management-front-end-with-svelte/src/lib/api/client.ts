// Lean fetch wrapper for the Case Service.
//
// The service is a loco.rs app: handlers return RAW JSON (no
// {success,data,error} envelope), so this client returns the parsed
// body directly and throws ApiError on non-2xx.
//
// Auth is handled by the same-origin BFF proxy (it injects a server-
// exchanged PASETO), so this client attaches no bearer of its own — only
// an explicit per-request `token` override, if ever passed. See
// `agents/share/authentication-sessions.md`.

/**
 * Construction-time configuration for {@link ApiClient}.
 */
export interface ClientOptions {
  /** Absolute origin (and optional prefix) of the Case Service, e.g. `http://localhost:5150`. Trailing slashes are trimmed. */
  baseUrl: string;
  /** Override the `fetch` implementation (SSR, tests, instrumentation). Defaults to the global `fetch`. */
  fetch?: typeof fetch;
}

/**
 * Per-request options shared by every verb on {@link ApiClient}.
 */
export interface RequestOptions {
  /** JSON request body; serialised with `JSON.stringify`. Omit for bodyless requests (GET/DELETE). */
  body?: unknown;
  /** Override the bearer for this request: a string sets the header, `null`/`""` suppresses it. Omit to send no bearer (the BFF injects one). */
  token?: string | null;
  /** Extra request headers, merged over (and able to override) the JSON defaults. */
  headers?: Record<string, string>;
  /** Abort signal for cancellation (e.g. component teardown). */
  signal?: AbortSignal;
}

/**
 * Error thrown for any non-2xx response. Carries the HTTP status and the
 * parsed (or raw) error body so callers can branch on them.
 */
export class ApiError extends Error {
  /** The HTTP status code of the failed response. */
  readonly status: number;
  /** The parsed JSON error body, or `undefined` when the body was empty / not JSON. */
  readonly body: unknown;

  /**
   * @param status HTTP status code of the failed response.
   * @param message Human-readable message (server-supplied where possible, else `HTTP <status>`).
   * @param body Parsed error body, if any.
   */
  constructor(status: number, message: string, body?: unknown) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.body = body;
  }

  /** True when the status is 401 — the session token is missing/expired. */
  get isUnauthorized(): boolean {
    return this.status === 401;
  }
  /** True when the status is 400 — a malformed request. */
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
 * Lean fetch wrapper for the Case Service's raw-JSON (no envelope) API.
 * Resolves to the parsed response body, or throws {@link ApiError} on any
 * non-2xx status. Holds no request state, so a single instance is reusable.
 */
export class ApiClient {
  /** Normalised base URL (trailing slashes stripped). */
  private readonly baseUrl: string;
  /** The `fetch` implementation to use for every request. */
  private readonly fetchFn: typeof fetch;

  /**
   * @param options Base URL, plus optional `fetch` seam.
   */
  constructor(options: ClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
  }

  /**
   * Issue a GET request.
   * @typeParam T Expected shape of the parsed response body.
   * @param path Request path (absolute or leading-slash-optional).
   * @param opts Optional headers/token/signal.
   * @returns The parsed response body.
   * @throws {ApiError} on any non-2xx response.
   */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /**
   * Issue a POST request.
   * @typeParam T Expected shape of the parsed response body.
   * @param path Request path.
   * @param opts Options; usually carries `body`.
   * @returns The parsed response body.
   * @throws {ApiError} on any non-2xx response.
   */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }
  /**
   * Issue a PUT request.
   * @typeParam T Expected shape of the parsed response body.
   * @param path Request path.
   * @param opts Options; usually carries `body`.
   * @returns The parsed response body.
   * @throws {ApiError} on any non-2xx response.
   */
  put<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PUT", path, opts);
  }
  /**
   * Issue a PATCH request (partial update, e.g. a board status move).
   * @typeParam T Expected shape of the parsed response body.
   */
  patch<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PATCH", path, opts);
  }
  /**
   * Issue a DELETE request. Defaults `T` to `void` since soft-delete
   * returns an empty body.
   * @typeParam T Expected shape of the parsed response body.
   * @param path Request path.
   * @param opts Optional headers/token/signal.
   * @returns The parsed response body (typically `undefined`).
   * @throws {ApiError} on any non-2xx response.
   */
  delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("DELETE", path, opts);
  }

  /**
   * Core request pipeline shared by all verbs: assemble headers, resolve
   * the bearer token, send, then parse the body and map non-2xx to
   * {@link ApiError}.
   * @typeParam T Expected shape of the parsed response body.
   * @throws {ApiError} on any non-2xx response.
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
    // JSON defaults first, then spread caller overrides so they win.
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

    // Resolve relative paths against `baseUrl`; the trailing slash on the
    // base keeps `URL` from dropping any base path segment.
    const url = new URL(
      path.startsWith("/") ? path : `/${path}`,
      `${this.baseUrl}/`,
    ).toString();
    const response = await this.fetchFn(url, init);

    // Read as text first so we can handle empty bodies (soft-delete) and
    // non-JSON error pages without a hard parse failure.
    const text = await response.text();
    let parsed: unknown = undefined;
    if (text.length > 0) {
      try {
        parsed = JSON.parse(text);
      } catch {
        // Non-JSON body: only surface it as an error on a failed
        // response (truncated); a non-JSON 2xx body stays `undefined`.
        if (!response.ok) {
          throw new ApiError(response.status, text.slice(0, 200));
        }
      }
    }

    if (!response.ok) {
      // Prefer a server-supplied message; otherwise a generic status line.
      const message = extractMessage(parsed) ?? `HTTP ${response.status}`;
      throw new ApiError(response.status, message, parsed);
    }
    return { data: parsed as T, response };
  }
}

/**
 * Best-effort extraction of a human-readable message from a parsed error
 * body, trying the common loco.rs/JSON keys in order.
 * @param body Parsed response body (any shape).
 * @returns The first string found under `error`/`message`/`description`, else `undefined`.
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
