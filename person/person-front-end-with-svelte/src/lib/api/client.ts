import type { ApiErrorBody, ApiResponse } from "./types.js";

/** Construction options for {@link ApiClient}. */
export interface ClientOptions {
  /** Base URL of the API; a trailing slash is stripped on construction. */
  baseUrl: string;
  /**
   * `fetch` implementation to use. Inject `event.fetch` from a SvelteKit
   * load function (for SSR + cookie forwarding) or a stub in tests.
   * Defaults to the global `fetch`.
   */
  fetch?: typeof fetch;
  /** Extra default headers merged into every request. */
  headers?: Record<string, string>;
}

/** Per-request options for an {@link ApiClient} call. */
export interface RequestOptions {
  /**
   * Query-string parameters. `undefined`/`null` values are skipped, so
   * optional params can be passed through without manual filtering.
   */
  query?: Record<string, string | number | boolean | undefined | null>;
  /**
   * Request body. JSON-serialized when present, **except** a `FormData`
   * body, which is passed through untouched for multipart uploads (see
   * {@link isFormDataBody}).
   */
  body?: unknown;
  /** Per-request headers, merged over the client's default headers. */
  headers?: Record<string, string>;
  /** Abort signal for cancellation (e.g. superseded search-as-you-type). */
  signal?: AbortSignal;
}

/**
 * Error thrown for any non-success API outcome — a non-2xx HTTP status or
 * an envelope with `success === false`.
 *
 * Carries the parsed {@link ApiErrorBody} so callers can branch on the
 * machine-readable `code` and HTTP `status` rather than parsing message
 * strings. The boolean getters cover the statuses the UI handles specially.
 */
export class ApiError extends Error {
  /** HTTP status code of the failed response. */
  readonly status: number;
  /** Machine-readable error code from the body, or `"UNKNOWN"`. */
  readonly code: string;
  /** Free-form structured details from the body (e.g. field errors). */
  readonly details: unknown;

  /**
   * @param status HTTP status code of the response.
   * @param body Parsed error body, or `null` when the response had none.
   * @param fallbackMessage Message to use when the body carries none
   *   (e.g. a non-JSON response). Falls back further to `HTTP <status>`.
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

  /** True for `404 Not Found` (record missing / soft-deleted). */
  get isNotFound(): boolean {
    return this.status === 404;
  }
  /** True for `409 Conflict` (duplicate detected on create). */
  get isConflict(): boolean {
    return this.status === 409;
  }
  /** True for `422 Unprocessable Entity` (server-side validation failure). */
  get isValidation(): boolean {
    return this.status === 422;
  }
}

/**
 * Whether a request body is `FormData` and must therefore bypass JSON
 * serialization.
 *
 * Guards on `typeof FormData` before the `instanceof`, since the global is
 * absent in some non-browser runtimes and a bare `instanceof` would throw
 * there rather than answering "no".
 *
 * @param body - The caller-supplied request body.
 * @returns `true` when the body is a `FormData` instance.
 */
export function isFormDataBody(body: unknown): body is FormData {
  return typeof FormData !== "undefined" && body instanceof FormData;
}

/**
 * Tiny `fetch` wrapper that understands the Person Service
 * {@link ApiResponse}`<T>` envelope and unwraps it to the bare `data`.
 *
 * Stateless aside from base URL + default headers, so it is cheap to
 * construct per page/component (the project deliberately avoids global
 * HTTP stores). `fetch` is injectable to support SSR load functions
 * (pass `event.fetch`) and tests. All failures surface as {@link ApiError}.
 */
export class ApiClient {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;
  private readonly defaultHeaders: Record<string, string>;

  /** @param options See {@link ClientOptions}. */
  constructor(options: ClientOptions) {
    // Strip trailing slashes so path joining in buildUrl is predictable.
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    // Bind the global fetch to avoid "illegal invocation" in some runtimes.
    this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
    this.defaultHeaders = {
      "content-type": "application/json",
      accept: "application/json",
      ...options.headers,
    };
  }

  /** Issue a GET and unwrap the response to `T`. @param path API path. */
  get<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, opts);
  }
  /** Issue a POST and unwrap the response to `T`. @param path API path. */
  post<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, opts);
  }
  /** Issue a PUT and unwrap the response to `T`. @param path API path. */
  put<T>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PUT", path, opts);
  }
  /** Issue a DELETE; defaults to `void` since the API returns 204. */
  delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("DELETE", path, opts);
  }

  /**
   * Core request pipeline shared by every verb: build the URL, send the
   * request, then classify the outcome.
   *
   * @returns The unwrapped envelope `data` as `T`.
   * @throws {ApiError} on a non-2xx status, a `success: false` envelope,
   *   or a body that is present but not valid JSON.
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
    const multipart = isFormDataBody(opts.body);
    if (multipart) {
      // `fetch` must set `content-type` itself here, because only it knows
      // the boundary token it generated. Leaving the client's default
      // `application/json` in place would produce a body the server cannot
      // parse (`400 BAD_MULTIPART`). Strip every casing, since a
      // per-request `headers` entry may have supplied its own.
      for (const key of Object.keys(headers)) {
        if (key.toLowerCase() === "content-type") delete headers[key];
      }
    }
    const init: RequestInit = { method, headers, signal: opts.signal };
    if (opts.body !== undefined) {
      init.body = multipart
        ? (opts.body as FormData)
        : JSON.stringify(opts.body);
    }

    const response = await this.fetchFn(url, init);

    // 204 No Content (e.g. DELETE) carries no body to unwrap.
    if (response.status === 204) {
      return undefined as T;
    }

    // Read as text first so we can give a useful error on non-JSON
    // bodies (e.g. an HTML error page from a proxy) and tolerate an
    // empty body on otherwise-successful responses.
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

    // Failure classification, in precedence order: HTTP status first,
    // then an application-level `success: false` envelope.
    if (!response.ok) {
      throw new ApiError(response.status, parsed?.error ?? null);
    }
    if (parsed && parsed.success === false) {
      throw new ApiError(response.status, parsed.error);
    }
    // Success: hand back the unwrapped payload (undefined if none).
    return (parsed?.data ?? undefined) as T;
  }

  /**
   * Join `path` onto the base URL and append non-empty query params.
   *
   * `path` is resolved as a *relative* reference (its leading slash is
   * stripped) against the base URL, which keeps its trailing slash so
   * `URL` appends rather than replaces its path — see the inline
   * comment below for why an absolute-path reference would silently
   * drop the base's own path. `undefined`/`null` query values are
   * omitted entirely.
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
