import { ApiClient } from "./client.js";
import type {
  AuditEntry,
  BatchDeduplicationRequest,
  BatchDeduplicationResponse,
  MatchRequest,
  MatchResult,
  MergeRequest,
  MergeResponse,
  ReviewDecision,
  ReviewQueueItem,
  ReviewQueueListResponse,
  ReviewStatus,
  Thing,
} from "./types.js";
import { API_BASE_URL } from "$lib/config.js";

/**
 * Query parameters for `GET /api/things/review-queue`.
 *
 * `status` filters to one stored disposition; omitting it (rather than
 * sending a literal `"all"`, which the service does not recognise and
 * answers `422 INVALID_STATUS` for) returns every status. `limit` caps
 * the page size (service default 100, hard cap 500) — there is no
 * `offset`; the endpoint offers no paging beyond the cap.
 */
export interface ReviewQueueOptions {
  status?: ReviewStatus;
  limit?: number;
}

/**
 * Parameters for a Thing search query.
 *
 * `q` is the search text; `fuzzy` enables edit-distance tolerance and
 * `phonetic` enables Soundex matching. `limit`/`offset` paginate, and
 * `mask_sensitive` requests server-side masking of sensitive fields.
 */
export interface SearchOptions {
  q: string;
  limit?: number;
  offset?: number;
  fuzzy?: boolean;
  phonetic?: boolean;
  mask_sensitive?: boolean;
}

/**
 * High-level, resource-bound client for the Thing Service REST API.
 *
 * Wraps an {@link ApiClient} and exposes one method per endpoint with
 * concrete request/response types, so pages never hand-build URLs. Stateless
 * beyond its injected client, so constructing one per page/component (the
 * project's "no global HTTP stores" rule) is intentional and cheap.
 */
// Resource-bound REST client for Thing Service. One instance per page
// is fine; the client is stateless aside from base URL + headers.
export class ThingRepository {
  /** @param http - The lower-level envelope-aware HTTP client to delegate to. */
  constructor(private readonly http: ApiClient) {}

  /**
   * Convenience factory building a repository against the configured
   * {@link API_BASE_URL}.
   *
   * @param fetchFn - Optional `fetch` to inject (pass `event.fetch` from a
   *   SvelteKit load function; omit in the browser to use the global).
   * @returns A ready-to-use {@link ThingRepository}.
   */
  static withFetch(fetchFn?: typeof fetch): ThingRepository {
    return new ThingRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /**
   * Full-text search for Things, normalising the response shape.
   *
   * WHY the normalisation: the service may return either a bare array or a
   * `{ items, total }` envelope depending on version/endpoint; this method
   * collapses both into a stable `{ items, total, limit, offset }` so
   * callers never branch.
   *
   * `total`/`limit`/`offset` prefer the family-wide `X-Total-Count`/
   * `X-Limit`/`X-Offset` response headers (`agents/share/restful.md`)
   * over anything the body itself carries — the headers are the
   * authoritative count of rows matching the query *ignoring*
   * `limit`/`offset` (T-28), where the body's own `total` (when
   * present at all) may just echo the page length. A service that
   * predates the headers still works: the fallbacks below make that
   * case behave exactly as it always did.
   *
   * @param opts - Query text and search/pagination flags.
   * @returns Matching things, the total count, and the limit/offset the
   *   service actually applied (which may differ from what was asked
   *   for, e.g. a clamped `limit`).
   * @throws {ApiError} On a failed request.
   */
  async search(opts: SearchOptions): Promise<{
    items: Thing[];
    total: number;
    limit: number;
    offset: number;
  }> {
    const { data, response } = await this.http.getWithHeaders<
      Thing[] | { items: Thing[]; total?: number }
    >("/api/things/search", {
      query: {
        q: opts.q,
        limit: opts.limit,
        offset: opts.offset,
        fuzzy: opts.fuzzy,
        phonetic: opts.phonetic,
        mask_sensitive: opts.mask_sensitive,
      },
    });
    const items = Array.isArray(data) ? data : data.items;
    const bodyTotal = Array.isArray(data) ? undefined : data.total;
    const header = (name: string): number | undefined => {
      const raw = response.headers.get(name);
      if (raw === null) return undefined;
      const value = Number(raw);
      return Number.isFinite(value) ? value : undefined;
    };
    return {
      items,
      total: header("x-total-count") ?? bodyTotal ?? items.length,
      limit: header("x-limit") ?? opts.limit ?? items.length,
      offset: header("x-offset") ?? opts.offset ?? 0,
    };
  }

  /**
   * Fetch a single Thing by id.
   * @throws {ApiError} `isNotFound` when no such Thing exists.
   */
  get(id: string): Promise<Thing> {
    return this.http.get<Thing>(`/api/things/${id}`);
  }

  /**
   * Create a new Thing.
   * @returns The persisted Thing (with server-assigned `id`).
   * @throws {ApiError} `isConflict` (409) when duplicates are detected —
   *   `details` carries the candidate {@link MatchResult}s; `isValidation`
   *   (422) on invalid input.
   */
  create(thing: Thing): Promise<Thing> {
    return this.http.post<Thing>("/api/things", { body: thing });
  }

  /**
   * Replace an existing Thing by id (full update).
   * @throws {ApiError} `isNotFound`/`isValidation` as applicable.
   */
  update(id: string, thing: Thing): Promise<Thing> {
    return this.http.put<Thing>(`/api/things/${id}`, { body: thing });
  }

  /**
   * Soft-delete a Thing (marks inactive, preserving the record + audit).
   * @throws {ApiError} `isNotFound` when no such Thing exists.
   */
  softDelete(id: string): Promise<void> {
    return this.http.delete<void>(`/api/things/${id}`);
  }

  /**
   * Run an ad-hoc match against the index for a candidate description.
   * @returns Scored candidates ordered by the service (best first).
   */
  match(request: MatchRequest): Promise<MatchResult[]> {
    return this.http.post<MatchResult[]>("/api/things/match", {
      body: request,
    });
  }

  /**
   * Check whether a candidate Thing would duplicate existing records,
   * without creating it. Accepts a partial Thing.
   */
  checkDuplicates(candidate: Partial<Thing>): Promise<MatchResult[]> {
    return this.http.post<MatchResult[]>("/api/things/check-duplicates", {
      body: candidate,
    });
  }

  /**
   * Merge a duplicate Thing into a surviving main Thing.
   * @returns The merge record plus the post-merge main Thing.
   */
  merge(request: MergeRequest): Promise<MergeResponse> {
    return this.http.post<MergeResponse>("/api/things/merge", {
      body: request,
    });
  }

  /**
   * Run a full-index deduplication scan.
   * @param request - Optional thresholds; defaults to server defaults.
   */
  deduplicate(
    request: BatchDeduplicationRequest = {},
  ): Promise<BatchDeduplicationResponse> {
    return this.http.post<BatchDeduplicationResponse>(
      "/api/things/deduplicate",
      { body: request },
    );
  }

  /**
   * Load the stored review queue (newest first).
   *
   * @param options - Optional status filter + page size (see
   *   {@link ReviewQueueOptions}).
   */
  listReviewQueue(
    options: ReviewQueueOptions = {},
  ): Promise<ReviewQueueItem[]> {
    return this.http
      .get<ReviewQueueListResponse>("/api/things/review-queue", {
        query: { status: options.status, limit: options.limit },
      })
      .then((response) => response.items);
  }

  /**
   * Decide a pending review item. Only `pending` items can be decided;
   * the service returns 422 for anything else (first writer wins).
   */
  decideReview(id: string, status: ReviewDecision): Promise<ReviewQueueItem> {
    return this.http.post<ReviewQueueItem>(
      `/api/things/review-queue/${id}/decision`,
      { body: { status } },
    );
  }

  /** Fetch a Thing with sensitive fields masked (privacy view). */
  masked(id: string): Promise<Thing> {
    return this.http.get<Thing>(`/api/things/${id}/masked`);
  }

  /**
   * Export all data held for a Thing for a GDPR subject-access request.
   * @returns An opaque export payload (shape is endpoint-defined).
   */
  exportGdpr(id: string): Promise<unknown> {
    return this.http.get<unknown>(`/api/things/${id}/export`);
  }

  /**
   * Fetch the audit history for a single Thing.
   * @param limit - Max entries to return (default 50).
   */
  audit(id: string, limit = 50): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>(`/api/things/${id}/audit`, {
      query: { limit },
    });
  }

  /**
   * Fetch the most recent system-wide audit entries (dashboard feed).
   * @param limit - Max entries to return (default 50).
   */
  recentAudit(limit = 50): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>("/api/audit/recent", {
      query: { limit },
    });
  }

  /** Liveness/health probe; resolves to the service's status string. */
  health(): Promise<{ status: string }> {
    return this.http.get<{ status: string }>("/api/health");
  }
}
