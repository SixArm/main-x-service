import type { ApiErrorBody, ApiResponse } from "./types.js";

export interface ClientOptions {
    baseUrl: string;
    fetch?: typeof fetch;
    headers?: Record<string, string>;
}

export interface RequestOptions {
    query?: Record<string, string | number | boolean | undefined | null>;
    body?: unknown;
    headers?: Record<string, string>;
    signal?: AbortSignal;
}

export class ApiError extends Error {
    readonly status: number;
    readonly code: string;
    readonly details: unknown;

    constructor(status: number, body: ApiErrorBody | null, fallbackMessage?: string) {
        super(body?.message ?? fallbackMessage ?? `HTTP ${status}`);
        this.name = "ApiError";
        this.status = status;
        this.code = body?.code ?? "UNKNOWN";
        this.details = body?.details;
    }

    get isNotFound(): boolean { return this.status === 404; }
    get isConflict(): boolean { return this.status === 409; }
    get isValidation(): boolean { return this.status === 422; }
}

// Tiny fetch wrapper that understands the Event Service ApiResponse<T>
// envelope. Allows fetch injection for SSR load functions (use
// `event.fetch`) and tests.
export class ApiClient {
    private readonly baseUrl: string;
    private readonly fetchFn: typeof fetch;
    private readonly defaultHeaders: Record<string, string>;

    constructor(options: ClientOptions) {
        this.baseUrl = options.baseUrl.replace(/\/+$/, "");
        this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
        this.defaultHeaders = {
            "content-type": "application/json",
            "accept": "application/json",
            ...options.headers,
        };
    }

    get<T>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("GET", path, opts);
    }
    post<T>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("POST", path, opts);
    }
    put<T>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("PUT", path, opts);
    }
    delete<T = void>(path: string, opts?: RequestOptions): Promise<T> {
        return this.request<T>("DELETE", path, opts);
    }

    private async request<T>(method: string, path: string, opts: RequestOptions = {}): Promise<T> {
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

        if (response.status === 204) {
            return undefined as T;
        }

        let parsed: ApiResponse<T> | null = null;
        const text = await response.text();
        if (text.length > 0) {
            try {
                parsed = JSON.parse(text) as ApiResponse<T>;
            } catch {
                throw new ApiError(response.status, null, `Non-JSON response: ${text.slice(0, 200)}`);
            }
        }

        if (!response.ok) {
            throw new ApiError(response.status, parsed?.error ?? null);
        }
        if (parsed && parsed.success === false) {
            throw new ApiError(response.status, parsed.error);
        }
        return (parsed?.data ?? undefined) as T;
    }

    private buildUrl(path: string, query?: RequestOptions["query"]): string {
        const url = new URL(path.startsWith("/") ? path : `/${path}`, `${this.baseUrl}/`);
        if (query) {
            for (const [k, v] of Object.entries(query)) {
                if (v === undefined || v === null) continue;
                url.searchParams.set(k, String(v));
            }
        }
        return url.toString();
    }
}
