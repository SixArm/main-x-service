// Resource-bound wrapper over ApiClient for the case endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { Page, PageRequest } from "./client";
import type {
  Case,
  CaseRef,
  MergeRecordRow,
  MergeRequest,
  MergeResponse,
  ScoredRef,
} from "./types";

/**
 * Resource-bound facade over {@link ApiClient} mapping each Case Service
 * REST endpoint to a typed method. The single source of endpoint paths for
 * the UI, so route components never hand-build URLs.
 */
export class CaseRepository {
  /**
   * @param http The underlying client (carries base URL, fetch, auth).
   */
  constructor(private readonly http: ApiClient) {}

  /**
   * Convenience constructor wiring an {@link ApiClient} at the configured
   * {@link API_BASE_URL}. Used by route components.
   * @param fetchFn Optional `fetch` override (e.g. SvelteKit's `load` fetch).
   * @returns A repository ready to call the live service.
   */
  static withFetch(fetchFn?: typeof fetch): CaseRepository {
    return new CaseRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /**
   * List all cases. `GET /api/cases`.
   * @returns Lightweight `{pid, title}` refs for the list page.
   */
  list(): Promise<CaseRef[]> {
    return this.http.get<CaseRef[]>("/api/cases");
  }

  /**
   * `GET /api/cases?limit=&offset=` — one page, with the total.
   * @param page The window; omitted values leave the service's defaults.
   */
  listPage(page: PageRequest = {}): Promise<Page<CaseRef>> {
    return this.http.getPage<CaseRef>("/api/cases", page);
  }

  /**
   * Fetch one full case by persistent id. `GET /api/cases/{pid}`.
   * @param pid Persistent id (URL-encoded into the path).
   * @returns The full {@link Case} record.
   */
  get(pid: string): Promise<Case> {
    return this.http.get<Case>(`/api/cases/${encodeURIComponent(pid)}`);
  }

  /**
   * Create a case. `POST /api/cases`.
   * @param record The case payload (matcher `Case` shape).
   * @returns The created `{pid, title}` ref.
   */
  create(record: Case): Promise<CaseRef> {
    return this.http.post<CaseRef>("/api/cases", { body: record });
  }

  /**
   * Replace a case. `PUT /api/cases/{pid}`.
   * @param pid Persistent id (URL-encoded into the path).
   * @param record The full replacement payload.
   * @returns The updated `{pid, title}` ref.
   */
  update(pid: string, record: Case): Promise<CaseRef> {
    return this.http.put<CaseRef>(`/api/cases/${encodeURIComponent(pid)}`, {
      body: record,
    });
  }

  /**
   * Soft-delete a case. `DELETE /api/cases/{pid}`. The service returns an
   * empty body, hence `void`.
   * @param pid Persistent id (URL-encoded into the path).
   */
  remove(pid: string): Promise<void> {
    return this.http.delete(`/api/cases/${encodeURIComponent(pid)}`);
  }

  /**
   * Match a query against the stored cases. `POST /api/cases/check-duplicates`.
   * Note the endpoint is `check-duplicates` (not `/duplicates`); a unit test
   * pins this to guard against regression.
   * @param query A case payload to score against existing records.
   * @returns Scored candidate refs (score + confidence + match flag).
   */
  checkDuplicates(query: Case): Promise<ScoredRef[]> {
    return this.http.post<ScoredRef[]>("/api/cases/check-duplicates", {
      body: query,
    });
  }

  /**
   * Fold a confirmed duplicate into a surviving case.
   * `POST /api/cases/merge`. Destructive: the duplicate is soft-deleted.
   * @param request Which duplicate folds into which survivor, and why.
   * @returns The two pids plus the survivor's merged payload. The service
   *   returns these inline — there is no `merge_record` wrapper.
   */
  merge(request: MergeRequest): Promise<MergeResponse> {
    return this.http.post<MergeResponse>("/api/cases/merge", {
      body: request,
    });
  }

  /**
   * Recent merge-history rows, newest first (the service caps at 100).
   * `GET /api/cases/merges/recent`.
   * @returns Raw `merge_records` rows.
   */
  recentMerges(): Promise<MergeRecordRow[]> {
    return this.http.get<MergeRecordRow[]>("/api/cases/merges/recent");
  }
}
