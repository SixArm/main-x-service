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

/**
 * Parameters for {@link EventRepository.search}. `q` is the free-text
 * query; the rest narrow results (pagination, fuzzy matching, date window,
 * status/type facets) and `mask_sensitive` redacts protected fields.
 */
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

/**
 * Typed REST client for the Event Service, one method per endpoint. All
 * paths are mounted under `/api/v1/` (see the service's restful.md).
 * Construct one per page/component — there is no global HTTP store.
 */
// REST client for Event Service. Endpoints are mounted under
// `/api/v1/` — see event-service-rust-crate/AGENTS/restful.md.
export class EventRepository {
    /** @param http - The underlying {@link ApiClient} used for all calls. */
    constructor(private readonly http: ApiClient) {}

    /**
     * Convenience factory that builds a repository against the configured
     * {@link API_BASE_URL}.
     * @param fetchFn - Optional fetch override (pass `event.fetch` under SSR).
     */
    static withFetch(fetchFn?: typeof fetch): EventRepository {
        return new EventRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    /**
     * Full-text search for events.
     *
     * Normalizes the two response shapes the service may return (a bare
     * array, or an `{ items, total }` object) into a consistent
     * `{ items, total }` result.
     *
     * @param opts - Query and filter parameters.
     * @returns The matching events plus a total count for pagination.
     * @throws {ApiError} On a non-success response.
     */
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
        // Service may return a bare array (no total) or an envelope object.
        if (Array.isArray(data)) return { items: data, total: data.length };
        return { items: data.items, total: data.total ?? data.items.length };
    }

    /** Fetch a single event by id. @throws {ApiError} 404 when absent. */
    get(id: string): Promise<Event> { return this.http.get<Event>(`/api/v1/events/${id}`); }
    /** Create a new event. @throws {ApiError} 409 on duplicate, 422 on validation error. */
    create(event: Event): Promise<Event> { return this.http.post<Event>("/api/v1/events", { body: event }); }
    /** Replace an existing event by id. @throws {ApiError} 422 on validation error. */
    update(id: string, event: Event): Promise<Event> { return this.http.put<Event>(`/api/v1/events/${id}`, { body: event }); }
    /** Soft-delete (deactivate) an event by id. */
    softDelete(id: string): Promise<void> { return this.http.delete<void>(`/api/v1/events/${id}`); }
    /** Score existing events against the given attributes; returns ranked candidates. */
    match(request: MatchRequest): Promise<MatchResult[]> { return this.http.post<MatchResult[]>("/api/v1/events/match", { body: request }); }
    /** Pre-create duplicate check: score a candidate event without persisting it. */
    checkDuplicates(candidate: Partial<Event>): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/v1/events/check-duplicates", { body: candidate });
    }
    /** Merge a duplicate event into a surviving one; returns the merge record + survivor. */
    merge(request: MergeRequest): Promise<MergeResponse> { return this.http.post<MergeResponse>("/api/v1/events/merge", { body: request }); }
    /** Fetch an event with sensitive fields redacted (privacy-masked view). */
    masked(id: string): Promise<Event> { return this.http.get<Event>(`/api/v1/events/${id}/masked`); }
    /** GDPR data export for one event; shape is service-defined, hence `unknown`. */
    exportGdpr(id: string): Promise<unknown> { return this.http.get<unknown>(`/api/v1/events/${id}/export`); }
    /** Fetch the audit trail for one event (most recent first), capped at `limit`. */
    audit(id: string, limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>(`/api/v1/events/${id}/audit`, { query: { limit } });
    }
    /** Fetch the system-wide recent audit feed across all events, capped at `limit`. */
    recentAudit(limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>("/api/v1/audit/recent", { query: { limit } });
    }
    /** Liveness/health probe for the service. */
    health(): Promise<{ status: string }> {
        return this.http.get<{ status: string }>("/api/v1/health");
    }
}
