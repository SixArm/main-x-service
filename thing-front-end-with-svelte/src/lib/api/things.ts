import { ApiClient } from "./client.js";
import type {
    AuditEntry,
    BatchDeduplicationRequest,
    BatchDeduplicationResponse,
    MatchRequest,
    MatchResult,
    MergeRequest,
    MergeResponse,
    Thing,
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

// Resource-bound REST client for Thing Service. One instance per page
// is fine; the client is stateless aside from base URL + headers.
export class ThingRepository {
    constructor(private readonly http: ApiClient) {}

    static withFetch(fetchFn?: typeof fetch): ThingRepository {
        return new ThingRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    async search(opts: SearchOptions): Promise<{ items: Thing[]; total: number }> {
        const data = await this.http.get<Thing[] | { items: Thing[]; total?: number }>(
            "/api/things/search",
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

    get(id: string): Promise<Thing> {
        return this.http.get<Thing>(`/api/things/${id}`);
    }

    create(thing: Thing): Promise<Thing> {
        return this.http.post<Thing>("/api/things", { body: thing });
    }

    update(id: string, thing: Thing): Promise<Thing> {
        return this.http.put<Thing>(`/api/things/${id}`, { body: thing });
    }

    softDelete(id: string): Promise<void> {
        return this.http.delete<void>(`/api/things/${id}`);
    }

    match(request: MatchRequest): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/things/match", { body: request });
    }

    checkDuplicates(candidate: Partial<Thing>): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/things/duplicates", { body: candidate });
    }

    merge(request: MergeRequest): Promise<MergeResponse> {
        return this.http.post<MergeResponse>("/api/things/merge", { body: request });
    }

    deduplicate(request: BatchDeduplicationRequest = {}): Promise<BatchDeduplicationResponse> {
        return this.http.post<BatchDeduplicationResponse>("/api/things/deduplicate", { body: request });
    }

    masked(id: string): Promise<Thing> {
        return this.http.get<Thing>(`/api/things/${id}/masked`);
    }

    exportGdpr(id: string): Promise<unknown> {
        return this.http.get<unknown>(`/api/things/${id}/export`);
    }

    audit(id: string, limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>(`/api/things/${id}/audit`, { query: { limit } });
    }

    recentAudit(limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>("/api/audit/recent", { query: { limit } });
    }

    health(): Promise<{ status: string }> {
        return this.http.get<{ status: string }>("/api/health");
    }
}
