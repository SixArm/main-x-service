// Wrapper over ApiClient for the unified `/api/plans` endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { Plan, PlanRef, ScoredRef } from "./types";

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
   * @param mainPid The survivor's pid.
   * @param duplicatePid The duplicate's pid (soft-deleted on success).
   * @param reason Optional operator note recorded in the merge history.
   * @returns The merge result `{main_pid, duplicate_pid, main}`.
   */
  merge(
    mainPid: string,
    duplicatePid: string,
    reason?: string,
  ): Promise<{ main_pid: string; duplicate_pid: string; main: Plan }> {
    return this.http.post(`${this.base()}/merge`, {
      body: { main_pid: mainPid, duplicate_pid: duplicatePid, reason },
    });
  }
}
