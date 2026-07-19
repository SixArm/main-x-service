import { ApiClient } from "./client.js";
import type {
  AuditEntry,
  BatchDeduplicationRequest,
  BatchDeduplicationResponse,
  MatchRequest,
  MatchResult,
  MergeRequest,
  MergeResponse,
  Place,
  ReviewDecision,
  ReviewQueueItem,
  ReviewQueueListResponse,
} from "./types.js";
import { API_BASE_URL } from "$lib/config.js";

/** Options for {@link PlaceRepository.search}. */
export interface SearchOptions {
  /** Full-text query string. */
  q: string;
  /** Max results per page. */
  limit?: number;
  /** Result offset for pagination. */
  offset?: number;
  /** Enable fuzzy (typo-tolerant) matching. */
  fuzzy?: boolean;
  /** Enable phonetic (Soundex) matching. */
  phonetic?: boolean;
  /** Mask sensitive fields in results. */
  mask_sensitive?: boolean;
}

// REST client for Place Service. Endpoint shape per
// place-service-with-loco/AGENTS/restful.md. Place Service uses
// `/check-duplicates` for duplicate-check.
/**
 * Typed REST client for the Place Service, wrapping an {@link ApiClient}.
 *
 * One repository is constructed per page/component (no global HTTP store,
 * per the project's ground rules), so each can carry its own injected
 * `fetch` (e.g. `event.fetch` in SSR load functions).
 */
export class PlaceRepository {
  /** @param http - Configured low-level client used for every call. */
  constructor(private readonly http: ApiClient) {}

  /**
   * Convenience constructor that builds an {@link ApiClient} against the
   * configured {@link API_BASE_URL}.
   * @param fetchFn - Optional fetch impl (pass `event.fetch` under SSR).
   * @returns A ready-to-use repository.
   */
  static withFetch(fetchFn?: typeof fetch): PlaceRepository {
    return new PlaceRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /**
   * Full-text search for places.
   *
   * Normalizes the two response shapes the service may return — a bare
   * `Place[]` or a `{ items, total }` envelope — into a single shape.
   * @param opts - Query string plus pagination/matching flags.
   * @returns The matched places and a total count.
   */
  async search(
    opts: SearchOptions,
  ): Promise<{ items: Place[]; total: number }> {
    const data = await this.http.get<
      Place[] | { items: Place[]; total?: number }
    >("/api/places/search", {
      query: {
        q: opts.q,
        limit: opts.limit,
        offset: opts.offset,
        fuzzy: opts.fuzzy,
        phonetic: opts.phonetic,
        mask_sensitive: opts.mask_sensitive,
      },
    });
    // Bare array: total is just the array length.
    if (Array.isArray(data)) return { items: data, total: data.length };
    // Enveloped: trust `total`, falling back to the page length.
    return { items: data.items, total: data.total ?? data.items.length };
  }

  /**
   * Fetch one place by id.
   * @throws {ApiError} `isNotFound` when the id is unknown.
   */
  get(id: string): Promise<Place> {
    return this.http.get<Place>(`/api/places/${id}`);
  }

  /**
   * Create a new place.
   * @throws {ApiError} `isValidation` (422) on invalid input, or
   *   `isConflict` (409) when a duplicate is detected on create.
   */
  create(place: Place): Promise<Place> {
    return this.http.post<Place>("/api/places", { body: place });
  }

  /**
   * Replace an existing place.
   * @throws {ApiError} `isValidation` (422) or `isNotFound` (404).
   */
  update(id: string, place: Place): Promise<Place> {
    return this.http.put<Place>(`/api/places/${id}`, { body: place });
  }

  /** Soft-delete a place (marks `is_deleted`; does not hard-delete). */
  softDelete(id: string): Promise<void> {
    return this.http.delete<void>(`/api/places/${id}`);
  }

  /**
   * Score a partial place against existing records.
   * @returns Candidates above the request threshold, ranked by score.
   */
  match(request: MatchRequest): Promise<MatchResult[]> {
    return this.http.post<MatchResult[]>("/api/places/match", {
      body: request,
    });
  }

  /**
   * Duplicate-check without persisting anything.
   *
   * NOTE: the Place Service exposes this at `/api/places/check-duplicates`
   * (hyphenated), distinct from `/match`; used pre-create to warn the
   * operator before a 409 would occur.
   * @param candidate - A partial place to check.
   * @returns Potential duplicate matches.
   */
  checkDuplicates(candidate: Partial<Place>): Promise<MatchResult[]> {
    return this.http.post<MatchResult[]>("/api/places/check-duplicates", {
      body: candidate,
    });
  }

  /** Merge a duplicate place into a surviving main place. */
  merge(request: MergeRequest): Promise<MergeResponse> {
    return this.http.post<MergeResponse>("/api/places/merge", {
      body: request,
    });
  }

  /**
   * Run a batch deduplication scan over the whole index.
   * @param request - Optional thresholds; defaults to an empty body.
   */
  deduplicate(
    request: BatchDeduplicationRequest = {},
  ): Promise<BatchDeduplicationResponse> {
    return this.http.post<BatchDeduplicationResponse>(
      "/api/places/deduplicate",
      { body: request },
    );
  }

  /** Load the stored review queue (newest first). */
  listReviewQueue(): Promise<ReviewQueueItem[]> {
    return this.http
      .get<ReviewQueueListResponse>("/api/places/review-queue")
      .then((response) => response.items);
  }

  /**
   * Decide a pending review item. Only `pending` items can be decided;
   * the service returns 422 for anything else (first writer wins).
   */
  decideReview(id: string, status: ReviewDecision): Promise<ReviewQueueItem> {
    return this.http.post<ReviewQueueItem>(
      `/api/places/review-queue/${id}/decision`,
      { body: { status } },
    );
  }

  /** Fetch a place with sensitive fields masked (privacy view). */
  masked(id: string): Promise<Place> {
    return this.http.get<Place>(`/api/places/${id}/masked`);
  }

  /** GDPR data export for a single place (shape is service-defined). */
  exportGdpr(id: string): Promise<unknown> {
    return this.http.get<unknown>(`/api/places/${id}/export`);
  }

  /**
   * Audit history for one place, most recent first.
   * @param limit - Max entries to return (default 50).
   */
  audit(id: string, limit = 50): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>(`/api/places/${id}/audit`, {
      query: { limit },
    });
  }

  /**
   * System-wide recent audit entries across all places.
   * @param limit - Max entries to return (default 50).
   */
  recentAudit(limit = 50): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>("/api/audit/recent", {
      query: { limit },
    });
  }

  /** Service health probe; returns the reported status string. */
  health(): Promise<{ status: string }> {
    return this.http.get<{ status: string }>("/api/health");
  }
}
