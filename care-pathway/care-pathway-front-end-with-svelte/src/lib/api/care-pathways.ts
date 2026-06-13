// Resource-bound wrapper over ApiClient for the care-pathway endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { CarePathway, PathwayRef, ScoredRef } from "./types";

export class CarePathwayRepository {
    constructor(private readonly http: ApiClient) {}

    static withFetch(fetchFn?: typeof fetch): CarePathwayRepository {
        return new CarePathwayRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    list(): Promise<PathwayRef[]> {
        return this.http.get<PathwayRef[]>("/api/care-pathways");
    }

    /// Case-insensitive name search (`ILIKE`, cap 50). `q` is URL-encoded.
    search(q: string): Promise<PathwayRef[]> {
        return this.http.get<PathwayRef[]>(
            `/api/care-pathways/search?q=${encodeURIComponent(q)}`,
        );
    }

    get(pid: string): Promise<CarePathway> {
        return this.http.get<CarePathway>(`/api/care-pathways/${encodeURIComponent(pid)}`);
    }

    create(pathway: CarePathway): Promise<PathwayRef> {
        return this.http.post<PathwayRef>("/api/care-pathways", { body: pathway });
    }

    update(pid: string, pathway: CarePathway): Promise<PathwayRef> {
        return this.http.put<PathwayRef>(`/api/care-pathways/${encodeURIComponent(pid)}`, {
            body: pathway,
        });
    }

    remove(pid: string): Promise<void> {
        return this.http.delete(`/api/care-pathways/${encodeURIComponent(pid)}`);
    }

    /// Match a query against the stored care pathways.
    checkDuplicates(query: CarePathway): Promise<ScoredRef[]> {
        return this.http.post<ScoredRef[]>("/api/care-pathways/check-duplicates", { body: query });
    }
}
