import { ApiClient } from "./client.js";
import type {
  AuditEntry,
  BatchDeduplicationRequest,
  BatchDeduplicationResponse,
  CreateLinkRequest,
  EntityLink,
  MatchRequest,
  MatchResult,
  MergeRequest,
  MergeResponse,
  ReviewDecision,
  ReviewQueueItem,
  ReviewQueueListResponse,
  ReviewStatus,
  Worker,
} from "./types.js";
import { API_BASE_URL } from "$lib/config.js";

/** Parameters for {@link WorkerRepository.search}. */
export interface SearchOptions {
  /** Full-text query string (use `"*"` to list all). */
  q: string;
  /** Max results to return. */
  limit?: number;
  /** Pagination offset. */
  offset?: number;
  /** Enable fuzzy (edit-distance) matching. */
  fuzzy?: boolean;
  /** Enable phonetic (Soundex) matching. */
  phonetic?: boolean;
  /** Mask sensitive fields (e.g. tax id) in the results. */
  mask_sensitive?: boolean;
}

/** Parameters for {@link WorkerRepository.listReviewQueue}. */
export interface ReviewQueueOptions {
  /**
   * Restrict to one disposition. Omit for every status — the endpoint
   * has no "all" token, so "all" is the *absence* of the parameter.
   */
  status?: ReviewStatus;
  /**
   * Page size. Defaults to 100 server-side and is capped at 500 there.
   * There is no `offset`, so this is the only pagination control.
   */
  limit?: number;
}

/**
 * Resource-bound REST client for the Worker Service — one method per
 * endpoint, wrapping an {@link ApiClient}. Per the project's no-global-store
 * rule, construct one per page/component (it's stateless aside from the
 * underlying client's base URL + headers).
 */
export class WorkerRepository {
  /** @param http - The configured {@link ApiClient} to issue requests with. */
  constructor(private readonly http: ApiClient) {}

  /**
   * Convenience constructor that builds an {@link ApiClient} pointed at
   * {@link API_BASE_URL}.
   * @param fetchFn - Optional fetch override (pass `event.fetch` in load
   *   functions, or a mock in tests).
   */
  static withFetch(fetchFn?: typeof fetch): WorkerRepository {
    return new WorkerRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /**
   * Full-text search for workers.
   *
   * Normalizes the two response shapes the service may emit (a bare array,
   * or `{ items, total }`) into a single `{ items, total }` result so
   * callers don't have to branch.
   *
   * @returns The matching workers plus a total count.
   * @throws {ApiError} On request failure.
   */
  async search(
    opts: SearchOptions,
  ): Promise<{ items: Worker[]; total: number }> {
    const data = await this.http.get<
      Worker[] | { items: Worker[]; total?: number }
    >("/api/workers/search", {
      query: {
        q: opts.q,
        limit: opts.limit,
        offset: opts.offset,
        fuzzy: opts.fuzzy,
        phonetic: opts.phonetic,
        mask_sensitive: opts.mask_sensitive,
      },
    });
    // Bare array → synthesize a total from its length; object → trust
    // its `total`, falling back to item count when absent.
    if (Array.isArray(data)) return { items: data, total: data.length };
    return { items: data.items, total: data.total ?? data.items.length };
  }

  /**
   * Fetch a single worker by id.
   * @throws {ApiError} `isNotFound` when the id is unknown.
   */
  get(id: string): Promise<Worker> {
    return this.http.get<Worker>(`/api/workers/${id}`);
  }

  /**
   * Create a new worker.
   * @throws {ApiError} `isConflict` (409) when duplicates are detected
   *   — `details` carries the candidate {@link MatchResult}s.
   */
  create(worker: Worker): Promise<Worker> {
    return this.http.post<Worker>("/api/workers", { body: worker });
  }

  /**
   * Replace an existing worker.
   * @throws {ApiError} On not-found or validation failure.
   */
  update(id: string, worker: Worker): Promise<Worker> {
    return this.http.put<Worker>(`/api/workers/${id}`, { body: worker });
  }

  /**
   * Soft-delete a worker (marks inactive; reversible only server-side).
   * @throws {ApiError} On request failure.
   */
  softDelete(id: string): Promise<void> {
    return this.http.delete<void>(`/api/workers/${id}`);
  }

  /**
   * Run an ad-hoc match query and return scored candidates.
   * @throws {ApiError} On request failure.
   */
  match(request: MatchRequest): Promise<MatchResult[]> {
    return this.http.post<MatchResult[]>("/api/workers/match", {
      body: request,
    });
  }

  /**
   * Check a candidate for duplicates without creating it (non-mutating
   * counterpart to the 409 path of {@link create}).
   * @throws {ApiError} On request failure.
   */
  checkDuplicates(candidate: Partial<Worker>): Promise<MatchResult[]> {
    return this.http.post<MatchResult[]>("/api/workers/check-duplicates", {
      body: candidate,
    });
  }

  /**
   * Merge a duplicate worker into a surviving main record.
   * @throws {ApiError} On request failure.
   */
  merge(request: MergeRequest): Promise<MergeResponse> {
    return this.http.post<MergeResponse>("/api/workers/merge", {
      body: request,
    });
  }

  /**
   * Run a whole-index batch deduplication scan.
   * @param request - Tuning knobs; defaults to an empty body.
   * @throws {ApiError} On request failure.
   */
  deduplicate(
    request: BatchDeduplicationRequest = {},
  ): Promise<BatchDeduplicationResponse> {
    return this.http.post<BatchDeduplicationResponse>(
      "/api/workers/deduplicate",
      { body: request },
    );
  }

  /**
   * Load the stored review queue (newest first).
   *
   * @param options - Optional `status` filter and page-size `limit`. There
   *   is no `offset` on this endpoint — `limit` is the whole of the
   *   pagination story. An unrecognised `status` gets a `422
   *   INVALID_STATUS` from the service — do not pass a literal `"all"`;
   *   omit the field instead.
   */
  listReviewQueue(
    options: ReviewQueueOptions = {},
  ): Promise<ReviewQueueItem[]> {
    return this.http
      .get<ReviewQueueListResponse>("/api/workers/review-queue", {
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
      `/api/workers/review-queue/${id}/decision`,
      { body: { status } },
    );
  }

  /**
   * List this worker's active outbound cross-service links (newest first).
   *
   * These are `entity_links` edges to records in *other* services — not the
   * within-service `Worker.links`.
   *
   * @throws {ApiError} `isNotFound` when the worker id is unknown.
   */
  listLinks(workerId: string): Promise<EntityLink[]> {
    return this.http.get<EntityLink[]>(`/api/workers/${workerId}/links`);
  }

  /**
   * Assert an outbound cross-service link from this worker. Idempotent
   * server-side: re-asserting the same edge upserts rather than duplicating.
   *
   * @throws {ApiError} `isValidation` (422) when the kind/target pair is not
   *   one worker may originate — `message` carries the server's reason.
   */
  createLink(
    workerId: string,
    request: CreateLinkRequest,
  ): Promise<EntityLink> {
    return this.http.post<EntityLink>(`/api/workers/${workerId}/links`, {
      body: request,
    });
  }

  /**
   * Withdraw (soft-delete) one outbound link. The service replies with an
   * empty envelope, so there is nothing to read back.
   *
   * @throws {ApiError} `isNotFound` when the link is unknown or already
   *   withdrawn.
   */
  async deleteLink(workerId: string, linkId: string): Promise<void> {
    await this.http.delete<unknown>(`/api/workers/${workerId}/links/${linkId}`);
  }

  /**
   * Fetch a worker with sensitive fields masked (privacy view).
   * @throws {ApiError} On request failure.
   */
  masked(id: string): Promise<Worker> {
    return this.http.get<Worker>(`/api/workers/${id}/masked`);
  }

  /**
   * GDPR data export for a worker. Untyped — the export shape is a full
   * record bundle defined by the service.
   * @throws {ApiError} On request failure.
   */
  exportGdpr(id: string): Promise<unknown> {
    return this.http.get<unknown>(`/api/workers/${id}/export`);
  }

  /**
   * Fetch the audit history for one worker, newest first.
   * @param limit - Max entries to return (default 50).
   * @throws {ApiError} On request failure.
   */
  audit(id: string, limit = 50): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>(`/api/workers/${id}/audit`, {
      query: { limit },
    });
  }

  /**
   * Fetch recent system-wide audit activity (for the dashboard).
   * @param limit - Max entries to return (default 50).
   * @throws {ApiError} On request failure.
   */
  recentAudit(limit = 50): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>("/api/audit/recent", {
      query: { limit },
    });
  }

  /**
   * Service health probe; used by the dashboard's status badge.
   * @throws {ApiError} (or a network error) when the service is down.
   */
  health(): Promise<{ status: string }> {
    return this.http.get<{ status: string }>("/api/health");
  }
}
