// Collection-bound wrapper over ApiClient for the work-item endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { Collection, ScoredRef, WorkItem, WorkItemRef } from "./types";

/**
 * Collection-bound facade over {@link ApiClient} mapping each Portfolio
 * Service REST endpoint to a typed method. Bound to one collection
 * (portfolios / projects / products / programs) so matching, listing, and
 * merge stay within a single collection — the single source of endpoint
 * paths for the UI.
 */
export class WorkItemRepository {
  /**
   * @param http The underlying client (carries base URL, fetch, auth).
   * @param collection The collection segment this repository is bound to.
   */
  constructor(
    private readonly http: ApiClient,
    private readonly collection: Collection,
  ) {}

  /**
   * Convenience constructor wiring an {@link ApiClient} at the configured
   * {@link API_BASE_URL}, bound to `collection`.
   * @param collection The collection segment.
   * @param fetchFn Optional `fetch` override (e.g. SvelteKit's `load` fetch).
   * @returns A repository ready to call the live service.
   */
  static withFetch(
    collection: Collection,
    fetchFn?: typeof fetch,
  ): WorkItemRepository {
    return new WorkItemRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
      collection,
    );
  }

  /** The base path for this collection. */
  private base(): string {
    return `/api/v1/${this.collection}`;
  }

  /**
   * List active work items in this collection. `GET /api/v1/{collection}`.
   * @param portfolioRef Optional parent portfolio pid (child collections)
   *   to roll up only that portfolio's children.
   * @returns Lightweight `{pid, name}` refs for the list page.
   */
  list(portfolioRef?: string): Promise<WorkItemRef[]> {
    const suffix = portfolioRef
      ? `?portfolio=${encodeURIComponent(portfolioRef)}`
      : "";
    return this.http.get<WorkItemRef[]>(`${this.base()}${suffix}`);
  }

  /**
   * Name search within this collection.
   * `GET /api/v1/{collection}/search?q=`.
   * @param q The case-insensitive substring to match.
   * @returns Matching refs.
   */
  search(q: string): Promise<WorkItemRef[]> {
    return this.http.get<WorkItemRef[]>(
      `${this.base()}/search?q=${encodeURIComponent(q)}`,
    );
  }

  /**
   * Fetch one full work item by persistent id.
   * `GET /api/v1/{collection}/{pid}`.
   * @param pid Persistent id (URL-encoded into the path).
   * @returns The full {@link WorkItem} record.
   */
  get(pid: string): Promise<WorkItem> {
    return this.http.get<WorkItem>(`${this.base()}/${encodeURIComponent(pid)}`);
  }

  /**
   * Create a work item. `POST /api/v1/{collection}`.
   * @param record The work-item payload (matcher `WorkItem` shape).
   * @returns The created `{pid, name}` ref.
   */
  create(record: WorkItem): Promise<WorkItemRef> {
    return this.http.post<WorkItemRef>(this.base(), { body: record });
  }

  /**
   * Replace a work item. `PUT /api/v1/{collection}/{pid}`.
   * @param pid Persistent id (URL-encoded into the path).
   * @param record The full replacement payload.
   * @returns The updated `{pid, name}` ref.
   */
  update(pid: string, record: WorkItem): Promise<WorkItemRef> {
    return this.http.put<WorkItemRef>(
      `${this.base()}/${encodeURIComponent(pid)}`,
      { body: record },
    );
  }

  /**
   * Soft-delete a work item. `DELETE /api/v1/{collection}/{pid}`. The
   * service returns an empty body, hence `void`.
   * @param pid Persistent id (URL-encoded into the path).
   */
  remove(pid: string): Promise<void> {
    return this.http.delete(`${this.base()}/${encodeURIComponent(pid)}`);
  }

  /**
   * Match a query against the stored work items in this collection.
   * `POST /api/v1/{collection}/check-duplicates`.
   * @param query A work-item payload to score against existing records.
   * @returns Scored candidate refs (score + confidence + match flag).
   */
  checkDuplicates(query: WorkItem): Promise<ScoredRef[]> {
    return this.http.post<ScoredRef[]>(`${this.base()}/check-duplicates`, {
      body: query,
    });
  }

  /**
   * Merge a confirmed duplicate into a surviving work item (same
   * collection). `POST /api/v1/{collection}/merge`.
   * @param mainPid The survivor's pid.
   * @param duplicatePid The duplicate's pid (soft-deleted on success).
   * @param reason Optional operator note recorded in the merge history.
   * @returns The merge result `{main_pid, duplicate_pid, main}`.
   */
  merge(
    mainPid: string,
    duplicatePid: string,
    reason?: string,
  ): Promise<{ main_pid: string; duplicate_pid: string; main: WorkItem }> {
    return this.http.post(`${this.base()}/merge`, {
      body: { main_pid: mainPid, duplicate_pid: duplicatePid, reason },
    });
  }
}
