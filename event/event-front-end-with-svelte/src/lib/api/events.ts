import { ApiClient } from "./client.js";
import type {
    AuditEntry,
    Event,
    MatchRequest,
    MatchResult,
    MergeRequest,
    MergeResponse,
} from "./types.js";
import { API_BASE_URL } from "$lib/config.js";

export interface SearchOptions {
    q: string;
    limit?: number;
    offset?: number;
    fuzzy?: boolean;
    mask_sensitive?: boolean;
    date_from?: string;
    date_to?: string;
    event_status?: string;
    event_type?: string;
}

// REST client for Event Service. Endpoints are mounted under
// `/api/v1/` — see event-service-rust-crate/AGENTS/restful.md.
export class EventRepository {
    constructor(private readonly http: ApiClient) {}

    static withFetch(fetchFn?: typeof fetch): EventRepository {
        return new EventRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    async search(opts: SearchOptions): Promise<{ items: Event[]; total: number }> {
        const data = await this.http.get<Event[] | { items: Event[]; total?: number }>(
            "/api/v1/events/search",
            {
                query: {
                    q: opts.q,
                    limit: opts.limit,
                    offset: opts.offset,
                    fuzzy: opts.fuzzy,
                    mask_sensitive: opts.mask_sensitive,
                    date_from: opts.date_from,
                    date_to: opts.date_to,
                    event_status: opts.event_status,
                    event_type: opts.event_type,
                },
            },
        );
        if (Array.isArray(data)) return { items: data, total: data.length };
        return { items: data.items, total: data.total ?? data.items.length };
    }

    get(id: string): Promise<Event> { return this.http.get<Event>(`/api/v1/events/${id}`); }
    create(event: Event): Promise<Event> { return this.http.post<Event>("/api/v1/events", { body: event }); }
    update(id: string, event: Event): Promise<Event> { return this.http.put<Event>(`/api/v1/events/${id}`, { body: event }); }
    softDelete(id: string): Promise<void> { return this.http.delete<void>(`/api/v1/events/${id}`); }
    match(request: MatchRequest): Promise<MatchResult[]> { return this.http.post<MatchResult[]>("/api/v1/events/match", { body: request }); }
    checkDuplicates(candidate: Partial<Event>): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/v1/events/check-duplicates", { body: candidate });
    }
    merge(request: MergeRequest): Promise<MergeResponse> { return this.http.post<MergeResponse>("/api/v1/events/merge", { body: request }); }
    masked(id: string): Promise<Event> { return this.http.get<Event>(`/api/v1/events/${id}/masked`); }
    exportGdpr(id: string): Promise<unknown> { return this.http.get<unknown>(`/api/v1/events/${id}/export`); }
    audit(id: string, limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>(`/api/v1/events/${id}/audit`, { query: { limit } });
    }
    recentAudit(limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>("/api/v1/audit/recent", { query: { limit } });
    }
    health(): Promise<{ status: string }> {
        return this.http.get<{ status: string }>("/api/v1/health");
    }
}
