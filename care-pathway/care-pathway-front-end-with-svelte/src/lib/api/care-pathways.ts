// Resource-bound wrapper over ApiClient for the care-pathway endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type {
    AuditEntry,
    CarePathway,
    MergeResult,
    PathwayEvent,
    PathwayRef,
    ScoredRef,
} from "./types";

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

    /// Merge a duplicate into a survivor. Both pids travel in the body;
    /// returns the survivor's refreshed record. `422` for equal pids,
    /// `404` for an unknown pid.
    merge(mainPid: string, duplicatePid: string, reason?: string): Promise<MergeResult> {
        const body: { main_pid: string; duplicate_pid: string; reason?: string } = {
            main_pid: mainPid,
            duplicate_pid: duplicatePid,
        };
        if (reason !== undefined) {
            body.reason = reason;
        }
        return this.http.post<MergeResult>("/api/care-pathways/merge", { body });
    }

    /// Audit trail for one care pathway, most-recent first. `pid` is
    /// URL-encoded.
    audit(pid: string): Promise<AuditEntry[]> {
        return this.http.get<AuditEntry[]>(
            `/api/care-pathways/${encodeURIComponent(pid)}/audit`,
        );
    }

    /// Recent system-wide CRUD/merge events from the service's in-memory
    /// stream. Returned roughly oldest-first (highest `seq` last); the UI
    /// sorts newest-first.
    recentEvents(): Promise<PathwayEvent[]> {
        return this.http.get<PathwayEvent[]>("/api/care-pathways/events/recent");
    }
}
