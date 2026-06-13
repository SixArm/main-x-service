// Lean fetch wrapper for the Case Service.
//
// The service is a loco.rs app: handlers return RAW JSON (no
// {success,data,error} envelope), so this client returns the parsed
// body directly and throws ApiError on non-2xx. A bearer token can be
// supplied per request for endpoints protected by JWT verification.

export interface ClientOptions {
    baseUrl: string;
    fetch?: typeof fetch;
}

export interface RequestOptions {
    body?: unknown;
    token?: string | null;
    headers?: Record<string, string>;
    signal?: AbortSignal;
}

export class ApiError extends Error {
    readonly status: number;
    readonly body: unknown;

    constructor(status: number, message: string, body?: unknown) {
        super(message);
        this.name = "ApiError";
        this.status = status;
        this.body = body;
    }

    get isUnauthorized(): boolean {
        return this.status === 401;
    }
    get isBadRequest(): boolean {
        return this.status === 400;
    }
}

export class ApiClient {
    private readonly baseUrl: string;
    private readonly fetchFn: typeof fetch;

    constructor(options: ClientOptions) {
        this.baseUrl = options.baseUrl.replace(/\/+$/, "");
        this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
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
        const headers: Record<string, string> = {
            "content-type": "application/json",
            accept: "application/json",
            ...opts.headers,
        };
        if (opts.token) {
            headers.authorization = `Bearer ${opts.token}`;
        }

        const init: RequestInit = { method, headers, signal: opts.signal };
        if (opts.body !== undefined) {
            init.body = JSON.stringify(opts.body);
        }

        const url = new URL(path.startsWith("/") ? path : `/${path}`, `${this.baseUrl}/`).toString();
        const response = await this.fetchFn(url, init);

        const text = await response.text();
        let parsed: unknown = undefined;
        if (text.length > 0) {
            try {
                parsed = JSON.parse(text);
            } catch {
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
