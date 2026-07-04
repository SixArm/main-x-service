import { ApiClient } from "./client.js";
import type {
  AuditEntry,
  BatchDeduplicationRequest,
  BatchDeduplicationResponse,
  MatchRequest,
  MatchResult,
  MergeRequest,
  MergeResponse,
  Course,
} from "./types.js";
import { API_BASE_URL } from "$lib/config.js";

/**
 * Query parameters for the full-text course search endpoint.
 *
 * @property q - Query string; an empty string lists all (the service
 *   falls back to `list` rather than running a wildcard search).
 * @property fuzzy - Enable edit-distance-tolerant matching.
 * @property phonetic - Enable Soundex phonetic matching.
 * @property mask_sensitive - Ask the service to redact sensitive fields.
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
 * Resource-bound REST client for the Course Service. Wraps a generic
 * {@link ApiClient} with one method per endpoint so callers work in
 * terms of {@link Course} domain types, not URLs. One instance per
 * page is fine; the client is stateless aside from base URL + headers.
 */
export class CourseRepository {
  /** @param http - Pre-configured low-level client used for every request. */
  constructor(private readonly http: ApiClient) {}

  /**
   * Build a repository wired to the configured {@link API_BASE_URL}.
   *
   * @param fetchFn - Fetch to inject for SSR (`event.fetch`) or tests;
   *   defaults to the global fetch.
   * @returns A ready-to-use repository.
   */
  static withFetch(fetchFn?: typeof fetch): CourseRepository {
    return new CourseRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /**
   * Full-text search for courses, normalising the service's response
   * into a stable `{ items, total }` shape.
   *
   * @param opts - Query and paging options (see {@link SearchOptions}).
   * @returns Matching courses plus the total result count.
   * @throws {ApiError} On a non-success response.
   */
  async search(
    opts: SearchOptions,
  ): Promise<{ items: Course[]; total: number }> {
    type SearchEnvelope =
      | Course[]
      | { items: Course[]; total?: number }
      | { courses: Course[]; total?: number };
    const data = await this.http.get<SearchEnvelope>("/api/courses/search", {
      query: {
        q: opts.q,
        limit: opts.limit,
        offset: opts.offset,
        fuzzy: opts.fuzzy,
        phonetic: opts.phonetic,
        mask_sensitive: opts.mask_sensitive,
      },
    });
    if (Array.isArray(data)) return { items: data, total: data.length };
    // Service emits {courses, total, …}; legacy shape was
    // {items, total}. Normalise both.
    const items = "items" in data ? data.items : data.courses;
    return { items, total: data.total ?? items.length };
  }

  /**
   * Fetch a single course by ID.
   * @throws {ApiError} 404 when no such course exists.
   */
  get(id: string): Promise<Course> {
    return this.http.get<Course>(`/api/courses/${id}`);
  }

  /**
   * Create a new course.
   * @returns The persisted course (with server-assigned `id`).
   * @throws {ApiError} 409 when duplicates are detected (candidates in
   *   `details`); 422 on validation failure.
   */
  create(course: Course): Promise<Course> {
    return this.http.post<Course>("/api/courses", { body: course });
  }

  /**
   * Replace an existing course (full update).
   * @throws {ApiError} 404 if absent; 422 on validation failure.
   */
  update(id: string, course: Course): Promise<Course> {
    return this.http.put<Course>(`/api/courses/${id}`, { body: course });
  }

  /**
   * Soft-delete a course (sets `deleted_at`; recoverable server-side).
   * @throws {ApiError} 404 if absent.
   */
  softDelete(id: string): Promise<void> {
    return this.http.delete<void>(`/api/courses/${id}`);
  }

  /**
   * Score a probe course against existing records without persisting.
   * @returns Blocked candidates sorted by descending score.
   * @throws {ApiError} 422 when the probe has no `name`.
   */
  match(request: MatchRequest): Promise<MatchResult[]> {
    return this.http.post<MatchResult[]>("/api/courses/match", {
      body: request,
    });
  }

  /**
   * Check a candidate for duplicates without creating it — the
   * pre-flight equivalent of the implicit check `create` runs.
   * @returns Potential duplicate matches (possibly empty).
   */
  checkDuplicates(candidate: Partial<Course>): Promise<MatchResult[]> {
    return this.http.post<MatchResult[]>("/api/courses/check-duplicates", {
      body: candidate,
    });
  }

  /**
   * Merge a duplicate course into a surviving main course.
   * @returns The merge record plus the updated surviving course.
   * @throws {ApiError} On invalid IDs or a service-side merge failure.
   */
  merge(request: MergeRequest): Promise<MergeResponse> {
    return this.http.post<MergeResponse>("/api/courses/merge", {
      body: request,
    });
  }

  /**
   * Run a batch deduplication scan over the whole index.
   * @param request - Threshold/auto-merge tuning (defaults to `{}`).
   * @returns Scan summary plus any pairs queued for review.
   */
  deduplicate(
    request: BatchDeduplicationRequest = {},
  ): Promise<BatchDeduplicationResponse> {
    return this.http.post<BatchDeduplicationResponse>(
      "/api/courses/deduplicate",
      { body: request },
    );
  }

  /** Fetch a course with sensitive fields redacted by the service. */
  masked(id: string): Promise<Course> {
    return this.http.get<Course>(`/api/courses/${id}/masked`);
  }

  /**
   * GDPR data export for a course. Shape is service-defined and
   * opaque to the UI, hence `unknown`.
   */
  exportGdpr(id: string): Promise<unknown> {
    return this.http.get<unknown>(`/api/courses/${id}/export`);
  }

  /**
   * Audit history for one course, newest first.
   * @param limit - Max entries to return (default 50).
   */
  audit(id: string, limit = 50): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>(`/api/courses/${id}/audit`, {
      query: { limit },
    });
  }

  /**
   * System-wide recent audit feed across all courses (dashboard).
   * @param limit - Max entries to return (default 50).
   */
  recentAudit(limit = 50): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>("/api/audit/recent", {
      query: { limit },
    });
  }

  /** Liveness probe; returns the service's `{ status }` health report. */
  health(): Promise<{ status: string }> {
    return this.http.get<{ status: string }>("/api/health");
  }
}
