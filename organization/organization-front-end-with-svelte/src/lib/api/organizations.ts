// Resource-bound wrapper over ApiClient for the organization endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { Organization, OrgRef, ScoredRef } from "./types";

export class OrganizationRepository {
    constructor(private readonly http: ApiClient) {}

    static withFetch(fetchFn?: typeof fetch): OrganizationRepository {
        return new OrganizationRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    list(): Promise<OrgRef[]> {
        return this.http.get<OrgRef[]>("/api/organizations");
    }

    get(pid: string): Promise<Organization> {
        return this.http.get<Organization>(`/api/organizations/${encodeURIComponent(pid)}`);
    }

    create(org: Organization): Promise<OrgRef> {
        return this.http.post<OrgRef>("/api/organizations", { body: org });
    }

    update(pid: string, org: Organization): Promise<OrgRef> {
        return this.http.put<OrgRef>(`/api/organizations/${encodeURIComponent(pid)}`, { body: org });
    }

    remove(pid: string): Promise<void> {
        return this.http.delete(`/api/organizations/${encodeURIComponent(pid)}`);
    }

    /// Match a query against the stored organizations.
    checkDuplicates(query: Organization): Promise<ScoredRef[]> {
        return this.http.post<ScoredRef[]>("/api/organizations/check-duplicates", { body: query });
    }
}
