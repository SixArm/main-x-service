// Resource-bound wrapper over ApiClient for the case endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { Case, CaseRef, ScoredRef } from "./types";

export class CaseRepository {
  constructor(private readonly http: ApiClient) {}

  static withFetch(fetchFn?: typeof fetch): CaseRepository {
    return new CaseRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  list(): Promise<CaseRef[]> {
    return this.http.get<CaseRef[]>("/api/cases");
  }

  get(pid: string): Promise<Case> {
    return this.http.get<Case>(`/api/cases/${encodeURIComponent(pid)}`);
  }

  create(record: Case): Promise<CaseRef> {
    return this.http.post<CaseRef>("/api/cases", { body: record });
  }

  update(pid: string, record: Case): Promise<CaseRef> {
    return this.http.put<CaseRef>(`/api/cases/${encodeURIComponent(pid)}`, {
      body: record,
    });
  }

  remove(pid: string): Promise<void> {
    return this.http.delete(`/api/cases/${encodeURIComponent(pid)}`);
  }

  /// Match a query against the stored cases.
  checkDuplicates(query: Case): Promise<ScoredRef[]> {
    return this.http.post<ScoredRef[]>("/api/cases/check-duplicates", {
      body: query,
    });
  }
}
