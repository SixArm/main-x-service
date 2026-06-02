import { ApiClient } from "./client.js";
import type {
    AuditEntry,
    BatchDeduplicationRequest,
    BatchDeduplicationResponse,
    MatchRequest,
    MatchResult,
    MergeRequest,
    MergeResponse,
    Worker,
} from "./types.js";
import { API_BASE_URL } from "$lib/config.js";

export interface SearchOptions {
    q: string;
    limit?: number;
    offset?: number;
    fuzzy?: boolean;
    phonetic?: boolean;
    mask_sensitive?: boolean;
}

// Resource-bound REST client for Worker Service. One instance per page
// is fine; the client is stateless aside from base URL + headers.
export class WorkerRepository {
    constructor(private readonly http: ApiClient) {}

    static withFetch(fetchFn?: typeof fetch): WorkerRepository {
        return new WorkerRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    async search(opts: SearchOptions): Promise<{ items: Worker[]; total: number }> {
        const data = await this.http.get<Worker[] | { items: Worker[]; total?: number }>(
            "/api/workers/search",
            {
                query: {
                    q: opts.q,
                    limit: opts.limit,
                    offset: opts.offset,
                    fuzzy: opts.fuzzy,
                    phonetic: opts.phonetic,
                    mask_sensitive: opts.mask_sensitive,
                },
            },
        );
        if (Array.isArray(data)) return { items: data, total: data.length };
        return { items: data.items, total: data.total ?? data.items.length };
    }

    get(id: string): Promise<Worker> {
        return this.http.get<Worker>(`/api/workers/${id}`);
    }

    create(worker: Worker): Promise<Worker> {
        return this.http.post<Worker>("/api/workers", { body: worker });
    }

    update(id: string, worker: Worker): Promise<Worker> {
        return this.http.put<Worker>(`/api/workers/${id}`, { body: worker });
    }

    softDelete(id: string): Promise<void> {
        return this.http.delete<void>(`/api/workers/${id}`);
    }

    match(request: MatchRequest): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/workers/match", { body: request });
    }

    checkDuplicates(candidate: Partial<Worker>): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/workers/check-duplicates", { body: candidate });
    }

    merge(request: MergeRequest): Promise<MergeResponse> {
        return this.http.post<MergeResponse>("/api/workers/merge", { body: request });
    }

    deduplicate(request: BatchDeduplicationRequest = {}): Promise<BatchDeduplicationResponse> {
        return this.http.post<BatchDeduplicationResponse>("/api/workers/deduplicate", { body: request });
    }

    masked(id: string): Promise<Worker> {
        return this.http.get<Worker>(`/api/workers/${id}/masked`);
    }

    exportGdpr(id: string): Promise<unknown> {
        return this.http.get<unknown>(`/api/workers/${id}/export`);
    }

    audit(id: string, limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>(`/api/workers/${id}/audit`, { query: { limit } });
    }

    recentAudit(limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>("/api/audit/recent", { query: { limit } });
    }

    health(): Promise<{ status: string }> {
        return this.http.get<{ status: string }>("/api/health");
    }
}
