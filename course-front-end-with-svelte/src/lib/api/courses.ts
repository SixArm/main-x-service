import { ApiClient } from "./client.js";
import type {
    AuditEntry,
    BatchDeduplicationRequest,
    BatchDeduplicationResponse,
    MatchRequest,
    MatchResult,
    MergeRequest,
    MergeResponse,
    Course,
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

// Resource-bound REST client for Course Service. One instance per page
// is fine; the client is stateless aside from base URL + headers.
export class CourseRepository {
    constructor(private readonly http: ApiClient) {}

    static withFetch(fetchFn?: typeof fetch): CourseRepository {
        return new CourseRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    async search(opts: SearchOptions): Promise<{ items: Course[]; total: number }> {
        type SearchEnvelope =
            | Course[]
            | { items: Course[]; total?: number }
            | { courses: Course[]; total?: number };
        const data = await this.http.get<SearchEnvelope>("/api/courses/search", {
            query: {
                q: opts.q,
                limit: opts.limit,
                offset: opts.offset,
                fuzzy: opts.fuzzy,
                phonetic: opts.phonetic,
                mask_sensitive: opts.mask_sensitive,
            },
        });
        if (Array.isArray(data)) return { items: data, total: data.length };
        // Service emits {courses, total, …}; legacy shape was
        // {items, total}. Normalise both.
        const items = "items" in data ? data.items : data.courses;
        return { items, total: data.total ?? items.length };
    }

    get(id: string): Promise<Course> {
        return this.http.get<Course>(`/api/courses/${id}`);
    }

    create(course: Course): Promise<Course> {
        return this.http.post<Course>("/api/courses", { body: course });
    }

    update(id: string, course: Course): Promise<Course> {
        return this.http.put<Course>(`/api/courses/${id}`, { body: course });
    }

    softDelete(id: string): Promise<void> {
        return this.http.delete<void>(`/api/courses/${id}`);
    }

    match(request: MatchRequest): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/courses/match", { body: request });
    }

    checkDuplicates(candidate: Partial<Course>): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/courses/check-duplicates", { body: candidate });
    }

    merge(request: MergeRequest): Promise<MergeResponse> {
        return this.http.post<MergeResponse>("/api/courses/merge", { body: request });
    }

    deduplicate(request: BatchDeduplicationRequest = {}): Promise<BatchDeduplicationResponse> {
        return this.http.post<BatchDeduplicationResponse>("/api/courses/deduplicate", { body: request });
    }

    masked(id: string): Promise<Course> {
        return this.http.get<Course>(`/api/courses/${id}/masked`);
    }

    exportGdpr(id: string): Promise<unknown> {
        return this.http.get<unknown>(`/api/courses/${id}/export`);
    }

    audit(id: string, limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>(`/api/courses/${id}/audit`, { query: { limit } });
    }

    recentAudit(limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>("/api/audit/recent", { query: { limit } });
    }

    health(): Promise<{ status: string }> {
        return this.http.get<{ status: string }>("/api/health");
    }
}
