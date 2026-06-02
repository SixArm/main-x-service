import { ApiClient } from "./client.js";
import type {
    AuditEntry,
    BatchDeduplicationRequest,
    BatchDeduplicationResponse,
    MatchRequest,
    MatchResult,
    MergeRequest,
    MergeResponse,
    Person,
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

// Resource-bound REST client for Person Service. One instance per page
// is fine; the client is stateless aside from base URL + headers.
export class PersonRepository {
    constructor(private readonly http: ApiClient) {}

    static withFetch(fetchFn?: typeof fetch): PersonRepository {
        return new PersonRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    async search(opts: SearchOptions): Promise<{ items: Person[]; total: number }> {
        const data = await this.http.get<Person[] | { items: Person[]; total?: number }>(
            "/api/persons/search",
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

    get(id: string): Promise<Person> {
        return this.http.get<Person>(`/api/persons/${id}`);
    }

    create(person: Person): Promise<Person> {
        return this.http.post<Person>("/api/persons", { body: person });
    }

    update(id: string, person: Person): Promise<Person> {
        return this.http.put<Person>(`/api/persons/${id}`, { body: person });
    }

    softDelete(id: string): Promise<void> {
        return this.http.delete<void>(`/api/persons/${id}`);
    }

    match(request: MatchRequest): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/persons/match", { body: request });
    }

    checkDuplicates(candidate: Partial<Person>): Promise<MatchResult[]> {
        return this.http.post<MatchResult[]>("/api/persons/check-duplicates", { body: candidate });
    }

    merge(request: MergeRequest): Promise<MergeResponse> {
        return this.http.post<MergeResponse>("/api/persons/merge", { body: request });
    }

    deduplicate(request: BatchDeduplicationRequest = {}): Promise<BatchDeduplicationResponse> {
        return this.http.post<BatchDeduplicationResponse>("/api/persons/deduplicate", { body: request });
    }

    masked(id: string): Promise<Person> {
        return this.http.get<Person>(`/api/persons/${id}/masked`);
    }

    exportGdpr(id: string): Promise<unknown> {
        return this.http.get<unknown>(`/api/persons/${id}/export`);
    }

    audit(id: string, limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>(`/api/persons/${id}/audit`, { query: { limit } });
    }

    recentAudit(limit = 50): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>("/api/audit/recent", { query: { limit } });
    }

    health(): Promise<{ status: string }> {
        return this.http.get<{ status: string }>("/api/health");
    }
}
