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
  Person,
  ReviewDecision,
  ReviewQueueItem,
  ReviewQueueListResponse,
} from "./types.js";
import { API_BASE_URL } from "$lib/config.js";

/** Parameters for {@link PersonRepository.search}. */
export interface SearchOptions {
  /** Free-text query string (name, identifier, address, …). */
  q: string;
  /** Max results to return (server-side pagination page size). */
  limit?: number;
  /** Result offset for pagination. */
  offset?: number;
  /** Enable fuzzy (edit-distance) matching. */
  fuzzy?: boolean;
  /** Enable phonetic (Soundex) name matching. */
  phonetic?: boolean;
  /** Ask the server to mask sensitive fields in the returned records. */
  mask_sensitive?: boolean;
}

/**
 * Resource-bound REST client for the Person Service — one method per
 * endpoint, returning already-unwrapped domain types.
 *
 * Stateless aside from the wrapped {@link ApiClient}, so constructing one
 * per page/component is intentional (the project avoids global HTTP
 * stores). All methods reject with {@link ApiError} on failure.
 */
export class PersonRepository {
  /** @param http The configured low-level client to delegate to. */
  constructor(private readonly http: ApiClient) {}

  /**
   * Convenience factory that builds a repository pointed at
   * {@link API_BASE_URL}.
   *
   * @param fetchFn Optional `fetch` to inject — pass `event.fetch` inside
   *   a SvelteKit load function so the request runs correctly under SSR.
   */
  static withFetch(fetchFn?: typeof fetch): PersonRepository {
    return new PersonRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /**
   * Full-text search for persons.
   *
   * Normalizes the service's response into a stable `{ items, total }`
   * shape regardless of which historical envelope the server returns.
   *
   * @returns Matching persons plus the total hit count for pagination.
   */
  async search(
    opts: SearchOptions,
  ): Promise<{ items: Person[]; total: number }> {
    // The endpoint has varied across service versions; accept all three
    // shapes so the front-end is resilient to that drift.
    type SearchEnvelope =
      | Person[]
      | { items: Person[]; total?: number }
      | { persons: Person[]; total?: number };
    const data = await this.http.get<SearchEnvelope>("/api/persons/search", {
      query: {
        q: opts.q,
        limit: opts.limit,
        offset: opts.offset,
        fuzzy: opts.fuzzy,
        phonetic: opts.phonetic,
        mask_sensitive: opts.mask_sensitive,
      },
    });
    // Bare array: no separate total, so derive it from the length.
    if (Array.isArray(data)) return { items: data, total: data.length };
    // Service emits `{persons, total, query, offset, limit}`; older
    // shape was `{items, total}`. Normalise both.
    const items = "items" in data ? data.items : data.persons;
    return { items, total: data.total ?? items.length };
  }

  /** Fetch one person by id. @throws {ApiError} 404 if not found. */
  get(id: string): Promise<Person> {
    return this.http.get<Person>(`/api/persons/${id}`);
  }

  /**
   * Create a new person.
   * @throws {ApiError} 409 when the server detects a duplicate,
   *   422 on validation failure.
   */
  create(person: Person): Promise<Person> {
    return this.http.post<Person>("/api/persons", { body: person });
  }

  /** Replace an existing person by id. @throws {ApiError} 422 on validation failure. */
  update(id: string, person: Person): Promise<Person> {
    return this.http.put<Person>(`/api/persons/${id}`, { body: person });
  }

  /** Soft-delete a person (marks inactive; preserves audit trail). */
  softDelete(id: string): Promise<void> {
    return this.http.delete<void>(`/api/persons/${id}`);
  }

  /** Score the index against a query record; returns ranked candidates. */
  match(request: MatchRequest): Promise<MatchResult[]> {
    return this.http.post<MatchResult[]>("/api/persons/match", {
      body: request,
    });
  }

  /**
   * Check whether a candidate person would collide with existing records,
   * without creating anything. Mirrors the duplicate detection that
   * {@link create} runs implicitly.
   */
  checkDuplicates(candidate: Partial<Person>): Promise<MatchResult[]> {
    return this.http.post<MatchResult[]>("/api/persons/check-duplicates", {
      body: candidate,
    });
  }

  /** Merge a confirmed duplicate into a surviving main record. */
  merge(request: MergeRequest): Promise<MergeResponse> {
    return this.http.post<MergeResponse>("/api/persons/merge", {
      body: request,
    });
  }

  /**
   * Run a batch deduplication scan over the whole index.
   * @param request Thresholds/limits; defaults to server-side defaults.
   */
  deduplicate(
    request: BatchDeduplicationRequest = {},
  ): Promise<BatchDeduplicationResponse> {
    return this.http.post<BatchDeduplicationResponse>(
      "/api/persons/deduplicate",
      { body: request },
    );
  }

  /** Load the stored review queue (newest first). */
  listReviewQueue(): Promise<ReviewQueueItem[]> {
    return this.http
      .get<ReviewQueueListResponse>("/api/persons/review-queue")
      .then((response) => response.items);
  }

  /**
   * Decide a pending review item. Only `pending` items can be decided;
   * the service returns 422 for anything else (first writer wins).
   */
  decideReview(id: string, status: ReviewDecision): Promise<ReviewQueueItem> {
    return this.http.post<ReviewQueueItem>(
      `/api/persons/review-queue/${id}/decision`,
      { body: { status } },
    );
  }

  /**
   * List this person's active outbound cross-service edges (newest
   * first). Withdrawn edges are excluded server-side.
   */
  listLinks(personId: string): Promise<EntityLink[]> {
    return this.http.get<EntityLink[]>(`/api/persons/${personId}/links`);
  }

  /**
   * Assert an outbound cross-service edge from this person. Idempotent
   * server-side: re-asserting the same `(kind, to_ref, valid_from)` tuple
   * updates the existing edge rather than duplicating it.
   *
   * @throws {ApiError} 422 when the kind is not one person originates or
   *   the target's entity type is wrong for the kind; 404 for an unknown
   *   person.
   */
  createLink(
    personId: string,
    request: CreateLinkRequest,
  ): Promise<EntityLink> {
    return this.http.post<EntityLink>(`/api/persons/${personId}/links`, {
      body: request,
    });
  }

  /**
   * Withdraw (soft-delete) one outbound edge. The service responds 200
   * with an empty payload, so there is nothing to read back.
   */
  deleteLink(personId: string, linkId: string): Promise<void> {
    return this.http
      .delete<unknown>(`/api/persons/${personId}/links/${linkId}`)
      .then(() => undefined);
  }

  /** Fetch a person with sensitive fields masked (privacy view). */
  masked(id: string): Promise<Person> {
    return this.http.get<Person>(`/api/persons/${id}/masked`);
  }

  /** GDPR data export for a person; shape is service-defined (`unknown`). */
  exportGdpr(id: string): Promise<unknown> {
    return this.http.get<unknown>(`/api/persons/${id}/export`);
  }

  /**
   * Audit history for one person.
   * @param limit Max entries (default 50), newest first.
   */
  audit(id: string, limit = 50): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>(`/api/persons/${id}/audit`, {
      query: { limit },
    });
  }

  /**
   * System-wide recent audit entries across all persons.
   * @param limit Max entries (default 50), newest first.
   */
  recentAudit(limit = 50): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>("/api/audit/recent", {
      query: { limit },
    });
  }

  /** Liveness/readiness probe for the service. */
  health(): Promise<{ status: string }> {
    return this.http.get<{ status: string }>("/api/health");
  }
}
