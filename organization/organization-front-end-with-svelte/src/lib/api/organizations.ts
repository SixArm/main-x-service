// Resource-bound wrapper over ApiClient for the organization endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { Organization, OrgRef, ScoredRef } from "./types";

/**
 * Resource-bound wrapper over {@link ApiClient}: one method per
 * organization endpoint, hiding the raw paths from the route components.
 * Every `pid` is URL-encoded since it appears in the path.
 */
export class OrganizationRepository {
    /** @param http The transport; share one per origin. */
    constructor(private readonly http: ApiClient) {}

    /**
     * Convenience constructor: builds a repository on a fresh
     * {@link ApiClient} pointed at {@link API_BASE_URL}.
     * @param fetchFn Optional `fetch` override (e.g. SvelteKit's load `fetch`).
     */
    static withFetch(fetchFn?: typeof fetch): OrganizationRepository {
        return new OrganizationRepository(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
    }

    /**
     * `GET /api/organizations`.
     * @returns The collection as `{pid, name}` refs.
     */
    list(): Promise<OrgRef[]> {
        return this.http.get<OrgRef[]>("/api/organizations");
    }

    /**
     * `GET /api/organizations/{pid}`.
     * @returns The full organization record.
     * @throws {@link ApiError} (404) when no record has that pid.
     */
    get(pid: string): Promise<Organization> {
        return this.http.get<Organization>(`/api/organizations/${encodeURIComponent(pid)}`);
    }

    /**
     * `POST /api/organizations`.
     * @param org The new record (only `name` is required).
     * @returns The created `{pid, name}` ref.
     */
    create(org: Organization): Promise<OrgRef> {
        return this.http.post<OrgRef>("/api/organizations", { body: org });
    }

    /**
     * `PUT /api/organizations/{pid}` — full replace of the record.
     * @returns The updated `{pid, name}` ref.
     */
    update(pid: string, org: Organization): Promise<OrgRef> {
        return this.http.put<OrgRef>(`/api/organizations/${encodeURIComponent(pid)}`, { body: org });
    }

    /**
     * `DELETE /api/organizations/{pid}` — soft delete (empty 200 body).
     */
    remove(pid: string): Promise<void> {
        return this.http.delete(`/api/organizations/${encodeURIComponent(pid)}`);
    }

    /// Match a query against the stored organizations.
    /**
     * `POST /api/organizations/check-duplicates` — score the given draft
     * organization against the stored records without persisting it.
     * @param query The candidate record to match.
     * @returns Scored refs (caller typically filters out the record's own pid).
     */
    checkDuplicates(query: Organization): Promise<ScoredRef[]> {
        return this.http.post<ScoredRef[]>("/api/organizations/check-duplicates", { body: query });
    }
}
