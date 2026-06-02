import { ApiClient } from "./client.js";
import type {
    AuditEntry,
    BatchDeduplicationRequest,
    BatchDeduplicationResponse,
    MatchRequest,
    MatchResult,
    MergeRequest,
    MergeResponse,
    Place,
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

// REST client for Place Service. Endpoint shape per
// place-service-rust-crate/AGENTS/restful.md. Note Place Service uses
// `/duplicates` (not `/check-duplicates`) for duplicate-check.
export class PlaceRepository {
    constructor(private readonly http: ApiClient) {}

    static withFetch(fetchFn?: typeof fetch): PlaceRepository {
        return new PlaceRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    async search(opts: SearchOptions): Promise<{ items: Place[]; total: number }> {
        const data = await this.http.get<Place[] | { items: Place[]; total?: number }>(
            "/api/places/search",
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

    get(id: string): Promise<Place> {
        return this.http.get<Place>(`/api/places/${id}`);
    }

    create(place: Place): Promise<Place> {
        return this.http.post<Place>("/api/places", { body: place });
    }

    update(id: string, place: Place): Promise<Place> {
        return this.http.put<Place>(`/api/places/${id}`, { body: place });
    }

    softDelete(id: string): Promise<void> {
        return this.http.delete<void>(`/api/places/${id}`);
    }

    match(request: MatchRequest): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/places/match", { body: request });
    }

    checkDuplicates(candidate: Partial<Place>): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/places/duplicates", { body: candidate });
    }

    merge(request: MergeRequest): Promise<MergeResponse> {
        return this.http.post<MergeResponse>("/api/places/merge", { body: request });
    }

    deduplicate(request: BatchDeduplicationRequest = {}): Promise<BatchDeduplicationResponse> {
        return this.http.post<BatchDeduplicationResponse>("/api/places/deduplicate", { body: request });
    }

    masked(id: string): Promise<Place> {
        return this.http.get<Place>(`/api/places/${id}/masked`);
    }

    exportGdpr(id: string): Promise<unknown> {
        return this.http.get<unknown>(`/api/places/${id}/export`);
    }

    audit(id: string, limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>(`/api/places/${id}/audit`, { query: { limit } });
    }

    recentAudit(limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>("/api/audit/recent", { query: { limit } });
    }

    health(): Promise<{ status: string }> {
        return this.http.get<{ status: string }>("/api/health");
    }
}
