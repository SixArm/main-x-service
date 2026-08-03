// Wrapper over ApiClient for the unified `/api/plans` endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { Page, PageRequest } from "./client";
import type {
  MergeRecordRow,
  MergeRequest,
  MergeResponse,
  Plan,
  PlanRef,
  ScoredRef,
} from "./types";

/**
 * Facade over {@link ApiClient} mapping each Portfolio Service REST
 * endpoint to a typed method. The service exposes ONE recursive
 * collection — `/api/plans` — so CRUD, matching, listing, and merge all
 * operate over that single collection (any plan may contain any other
 * plan via `parent_ref`). The single source of endpoint paths for the UI.
 */
export class PlanRepository {
  /**
   * @param http The underlying client (carries base URL, fetch, auth).
   */
  constructor(private readonly http: ApiClient) {}

  /**
   * Convenience constructor wiring an {@link ApiClient} at the configured
   * {@link API_BASE_URL}.
   * @param fetchFn Optional `fetch` override (e.g. SvelteKit's `load` fetch).
   * @returns A repository ready to call the live service.
   */
  static withFetch(fetchFn?: typeof fetch): PlanRepository {
    return new PlanRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /** The base path for the plans collection. */
  private base(): string {
    return `/api/plans`;
  }

  /**
   * List active plans. `GET /api/plans`.
   * @param parentRef Optional parent plan pid to roll up only that
   *   plan's direct children (`?parent=`).
   * @returns Lightweight `{pid, name}` refs for the list page.
   */
  list(parentRef?: string): Promise<PlanRef[]> {
    const suffix = parentRef ? `?parent=${encodeURIComponent(parentRef)}` : "";
    return this.http.get<PlanRef[]>(`${this.base()}${suffix}`);
  }

  /**
   * `GET /api/plans?limit=&offset=[&parent=]` — one page, with the total.
   *
   * The parent scope reaches the service's count as well as its page, so
   * a child listing's total describes the children rather than every
   * plan.
   * @param page The window; omitted values leave the service's defaults.
   */
  listPage(page: PageRequest = {}, parentRef?: string): Promise<Page<PlanRef>> {
    const suffix = parentRef ? `?parent=${encodeURIComponent(parentRef)}` : "";
    return this.http.getPage<PlanRef>(`${this.base()}${suffix}`, page);
  }

  /**
   * Name search. `GET /api/plans/search?q=`.
   * @param q The case-insensitive substring to match.
   * @returns Matching refs.
   */
  search(q: string): Promise<PlanRef[]> {
    return this.http.get<PlanRef[]>(
      `${this.base()}/search?q=${encodeURIComponent(q)}`,
    );
  }

  /**
   * Fetch one full plan by persistent id. `GET /api/plans/{pid}`.
   * @param pid Persistent id (URL-encoded into the path).
   * @returns The full {@link Plan} record.
   */
  get(pid: string): Promise<Plan> {
    return this.http.get<Plan>(`${this.base()}/${encodeURIComponent(pid)}`);
  }

  /**
   * Create a plan. `POST /api/plans`.
   * @param record The plan payload (matcher `Plan` shape).
   * @returns The created `{pid, name}` ref.
   */
  create(record: Plan): Promise<PlanRef> {
    return this.http.post<PlanRef>(this.base(), { body: record });
  }

  /**
   * Replace a plan. `PUT /api/plans/{pid}`.
   * @param pid Persistent id (URL-encoded into the path).
   * @param record The full replacement payload.
   * @returns The updated `{pid, name}` ref.
   */
  update(pid: string, record: Plan): Promise<PlanRef> {
    return this.http.put<PlanRef>(`${this.base()}/${encodeURIComponent(pid)}`, {
      body: record,
    });
  }

  /**
   * Soft-delete a plan. `DELETE /api/plans/{pid}`. The service returns an
   * empty body, hence `void`.
   * @param pid Persistent id (URL-encoded into the path).
   */
  remove(pid: string): Promise<void> {
    return this.http.delete(`${this.base()}/${encodeURIComponent(pid)}`);
  }

  /**
   * Match a query against the stored plans.
   * `POST /api/plans/check-duplicates`.
   * @param query A plan payload to score against existing records.
   * @returns Scored candidate refs (score + confidence + match flag).
   */
  checkDuplicates(query: Plan): Promise<ScoredRef[]> {
    return this.http.post<ScoredRef[]>(`${this.base()}/check-duplicates`, {
      body: query,
    });
  }

  /**
   * Merge a confirmed duplicate into a surviving plan.
   * `POST /api/plans/merge`.
   *
   * Destructive: the duplicate is soft-deleted and its data folded into
   * the survivor. The response carries the merged survivor itself, not a
   * merge-record wrapper — the history row is read back separately via
   * {@link recentMerges}.
   *
   * @param request The `{main_pid, duplicate_pid, reason?}` body.
   * @returns The merge result `{main_pid, duplicate_pid, main}`.
   */
  merge(request: MergeRequest): Promise<MergeResponse> {
    return this.http.post<MergeResponse>(`${this.base()}/merge`, {
      body: request,
    });
  }

  /**
   * Recent merge-history rows, newest first (service cap: 100).
   * `GET /api/plans/merges/recent`.
   * @returns The raw `merge_records` rows.
   */
  recentMerges(): Promise<MergeRecordRow[]> {
    return this.http.get<MergeRecordRow[]>(`${this.base()}/merges/recent`);
  }
}
