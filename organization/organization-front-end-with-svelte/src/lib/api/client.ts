// Lean fetch wrapper for the Organization Service.
//
// The service is a loco.rs app: handlers return RAW JSON (no
// {success,data,error} envelope), so this client returns the parsed
// body directly and throws ApiError on non-2xx.
//
// Auth is handled by the same-origin BFF proxy (it injects a
// server-exchanged PASETO server-side), so the browser holds no token and
// this client attaches no bearer of its own — only an explicit per-request
// `token` override, if ever passed. See
// `agents/share/authentication-sessions.md`.

/** Construction options for {@link ApiClient}. */
export interface ClientOptions {
  /** Service origin, e.g. `http://localhost:5150`; trailing slashes are trimmed. */
  baseUrl: string;
  /** Injectable `fetch` (SvelteKit's per-load `fetch`, or a test fake); defaults to the global. */
  fetch?: typeof fetch;
}

/** Per-call options for a single {@link ApiClient} request. */
export interface RequestOptions {
  /** Request payload; serialized to JSON. Omit for a bodyless request. */
  body?: unknown;
  /** Override the stored token. Omit to use the `auth` store; pass
   * `null`/`""` to suppress the header for this request. */
  token?: string | null;
  /** Extra request headers, merged over the JSON defaults. */
  headers?: Record<string, string>;
  /** Abort signal for cancellation. */
  signal?: AbortSignal;
}

/**
 * Error thrown for any non-2xx response. Carries the HTTP `status` and
 * the parsed `body` (when JSON) so callers can branch on the failure
 * without re-reading the response.
 */
export class ApiError extends Error {
  /** HTTP status code of the failing response. */
  readonly status: number;
  /** Parsed response body, or `undefined` when it was not JSON. */
  readonly body: unknown;

  /**
   * @param status HTTP status code.
   * @param message Human-readable message (server-supplied or synthesized).
   * @param body Parsed response body, when available.
   */
  constructor(status: number, message: string, body?: unknown) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.body = body;
  }

  /** True when the failure was `401 Unauthorized` (token missing/expired/rejected). */
  get isUnauthorized(): boolean {
    return this.status === 401;
  }
  /** True when the failure was `400 Bad Request`. */
  get isBadRequest(): boolean {
    return this.status === 400;
  }
}

/**
 * Lean JSON fetch wrapper over a single service origin. Each verb method
 * delegates to {@link request}, which attaches the bearer token, sends
 * the body as JSON, and either resolves the parsed body or throws an
 * {@link ApiError}.
 */
export class ApiClient {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;

  /** @param options Base URL and optional `fetch` override. */
  constructor(options: ClientOptions) {
    // Normalize the base so URL joining below never doubles a slash.
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
  }

  /**
   * Issue a `GET`.
   * @returns The parsed JSON body typed as `T`.
   * @throws {@link ApiError} on a non-2xx response.
   */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /**
   * Issue a `POST`.
   * @returns The parsed JSON body typed as `T`.
   * @throws {@link ApiError} on a non-2xx response.
   */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }
  /**
   * Issue a `PUT`.
   * @returns The parsed JSON body typed as `T`.
   * @throws {@link ApiError} on a non-2xx response.
   */
  put<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PUT", path, opts);
  }
  /**
   * Issue a `DELETE`. Defaults to `void` since soft-delete returns an
   * empty body.
   * @throws {@link ApiError} on a non-2xx response.
   */
  delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("DELETE", path, opts);
  }

  /**
   * Core request path shared by every verb: builds headers, attaches
   * the bearer token, sends the optional JSON body, reads the response
   * text, and parses it.
   * @returns The parsed JSON body typed as `T` (or `undefined` for an
   *   empty 2xx body).
   * @throws {@link ApiError} on any non-2xx response, with the
   *   server-supplied message when one can be extracted.
   */
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
    // The BFF proxy injects the bearer server-side; this client sends a
    // bearer only when one is explicitly passed for this request.
    if (opts.token) {
      headers.authorization = `Bearer ${opts.token}`;
    }

    const init: RequestInit = { method, headers, signal: opts.signal };
    if (opts.body !== undefined) {
      init.body = JSON.stringify(opts.body);
    }

    // Join path onto the base; the trailing slash on the base + a
    // leading slash on the path keeps `URL` from dropping a path segment.
    const url = new URL(
      path.startsWith("/") ? path : `/${path}`,
      `${this.baseUrl}/`,
    ).toString();
    const response = await this.fetchFn(url, init);

    // Read the raw text once; an empty body (e.g. soft-delete 200)
    // stays `undefined` rather than failing to parse.
    const text = await response.text();
    let parsed: unknown = undefined;
    if (text.length > 0) {
      try {
        parsed = JSON.parse(text);
      } catch {
        // Non-JSON error body (e.g. an HTML 500): surface a truncated
        // snippet as the message rather than throwing a parse error.
        if (!response.ok) {
          throw new ApiError(response.status, text.slice(0, 200));
        }
      }
    }

    if (!response.ok) {
      const message = extractMessage(parsed) ?? `HTTP ${response.status}`;
      throw new ApiError(response.status, message, parsed);
    }
    return parsed as T;
  }
}

/**
 * Pull a human-readable message out of a parsed error body, checking the
 * common loco.rs / generic keys (`error`, `message`, `description`) in
 * order. Returns `undefined` when none is present, so the caller can
 * fall back to an `HTTP <status>` message.
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
