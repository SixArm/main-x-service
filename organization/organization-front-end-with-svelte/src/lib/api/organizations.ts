// Resource-bound wrapper over ApiClient for the organization endpoints.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { Page, PageRequest } from "./client";
import type {
  BatchDeduplicationResponse,
  MatchRankResult,
  MergeRecordRow,
  MergeRequest,
  MergeResponse,
  Organization,
  OrgRef,
  ReviewDecision,
  ReviewQueueItem,
  ReviewQueueListResponse,
  ReviewStatus,
  ScoredRef,
} from "./types";

/** Parameters for {@link OrganizationRepository.listReviewQueue}. */
export interface ReviewQueueOptions {
  /**
   * Restrict to one disposition. Omit for every status — the endpoint
   * has no "all" token, so "all" is the *absence* of the parameter (an
   * unrecognised token, including the literal string `"all"`, is a `422`).
   */
  status?: ReviewStatus;
  /**
   * Page size. Defaults to 100 server-side and is capped at 500 there.
   * There is no `offset`, so this is the only pagination control.
   */
  limit?: number;
}

/**
 * Resource-bound wrapper over {@link ApiClient}: one method per
 * organization endpoint, hiding the raw paths from the route components.
 * Every `pid` is URL-encoded since it appears in the path.
 */
export class OrganizationRepository {
  /** @param http The transport; share one per origin. */
  constructor(private readonly http: ApiClient) {}

  /**
   * Convenience constructor: builds a repository on a fresh
   * {@link ApiClient} pointed at {@link API_BASE_URL}.
   * @param fetchFn Optional `fetch` override (e.g. SvelteKit's load `fetch`).
   */
  static withFetch(fetchFn?: typeof fetch): OrganizationRepository {
    return new OrganizationRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /**
   * `GET /api/organizations`.
   * @returns The collection as `{pid, name}` refs.
   */
  list(): Promise<OrgRef[]> {
    return this.http.get<OrgRef[]>("/api/organizations");
  }

  /**
   * `GET /api/organizations?limit=&offset=` — one page, with the total.
   * @param page The window; omitted values leave the service's defaults.
   * @returns The refs plus `total` / `limit` / `offset` as applied.
   */
  listPage(page: PageRequest = {}): Promise<Page<OrgRef>> {
    return this.http.getPage<OrgRef>("/api/organizations", page);
  }

  /**
   * `GET /api/organizations/{pid}`.
   * @returns The full organization record.
   * @throws {@link ApiError} (404) when no record has that pid.
   */
  get(pid: string): Promise<Organization> {
    return this.http.get<Organization>(
      `/api/organizations/${encodeURIComponent(pid)}`,
    );
  }

  /**
   * `GET /api/organizations/{pid}/masked` — the record with telephone,
   * email, street line, and fiscal identifiers redacted, regardless of
   * the caller's policy. Distinct from the `mask` obligation on
   * {@link get}: this is a caller *asking* for the redacted form (a
   * screen share, a support call, an export preview), not the
   * deployment deciding what a caller may see.
   * @returns The masked record.
   * @throws {@link ApiError} (404) when no record has that pid.
   */
  masked(pid: string): Promise<Organization> {
    return this.http.get<Organization>(
      `/api/organizations/${encodeURIComponent(pid)}/masked`,
    );
  }

  /**
   * `GET /api/organizations/{pid}/export` — the GDPR right-of-access
   * export envelope. Audited server-side on every call, including a
   * masked one. The shape is service-defined and never interpreted
   * here — callers only serialize and save what came back (see the
   * detail page's download action).
   * @throws {@link ApiError} (404) when no record has that pid.
   */
  exportGdpr(pid: string): Promise<unknown> {
    return this.http.get<unknown>(
      `/api/organizations/${encodeURIComponent(pid)}/export`,
    );
  }

  /**
   * `POST /api/organizations`.
   * @param org The new record (only `name` is required).
   * @returns The created `{pid, name}` ref.
   */
  create(org: Organization): Promise<OrgRef> {
    return this.http.post<OrgRef>("/api/organizations", { body: org });
  }

  /**
   * `PUT /api/organizations/{pid}` — full replace of the record.
   * @returns The updated `{pid, name}` ref.
   */
  update(pid: string, org: Organization): Promise<OrgRef> {
    return this.http.put<OrgRef>(
      `/api/organizations/${encodeURIComponent(pid)}`,
      { body: org },
    );
  }

  /**
   * `DELETE /api/organizations/{pid}` — soft delete (empty 200 body).
   */
  remove(pid: string): Promise<void> {
    return this.http.delete(`/api/organizations/${encodeURIComponent(pid)}`);
  }

  /// Match a query against the stored organizations.
  /**
   * `POST /api/organizations/check-duplicates` — score the given draft
   * organization against the stored records without persisting it.
   * @param query The candidate record to match.
   * @returns Scored refs (caller typically filters out the record's own pid).
   */
  checkDuplicates(query: Organization): Promise<ScoredRef[]> {
    return this.http.post<ScoredRef[]>("/api/organizations/check-duplicates", {
      body: query,
    });
  }

  /**
   * `POST /api/organizations/deduplicate` — batch-scan the stored
   * records pairwise and persist likely duplicates in the stored
   * review queue (a destructive-classed action; button-triggered only).
   * @returns The scan report over the STORED rows (stable item ids).
   */
  deduplicate(): Promise<BatchDeduplicationResponse> {
    return this.http.post<BatchDeduplicationResponse>(
      "/api/organizations/deduplicate",
      { body: {} },
    );
  }

  /**
   * `GET /api/organizations/review-queue[?status=&limit=]` — the stored
   * review queue, newest first.
   *
   * @param options Optional server-side `status` filter and page size.
   *   Both are omitted from the query string when absent, so a bare call
   *   keeps the endpoint's own defaults (all statuses, `limit` 100).
   * @throws {@link ApiError} 422 for a status token the service does not
   *   recognise.
   */
  listReviewQueue(
    options: ReviewQueueOptions = {},
  ): Promise<ReviewQueueItem[]> {
    const query = new URLSearchParams();
    if (options.status !== undefined) query.set("status", options.status);
    if (options.limit !== undefined) query.set("limit", String(options.limit));
    const suffix = query.toString();
    const path = suffix
      ? `/api/organizations/review-queue?${suffix}`
      : "/api/organizations/review-queue";
    return this.http
      .get<ReviewQueueListResponse>(path)
      .then((response) => response.items);
  }

  /**
   * `POST /api/organizations/match` — rank `candidates` against `query`
   * without persisting anything.
   *
   * Used by the `/review` comparison panel to obtain a **live** score
   * breakdown for a pending pair: the stored review-queue item never
   * carries `score_breakdown` on the wire (see `$lib/review`'s doc
   * comment), but this endpoint's `MatchResult.breakdown` does.
   *
   * @param query The organization to score against each candidate.
   * @param candidates The candidate organizations to rank.
   * @returns One `[candidateIndex, result]` pair per candidate,
   *   descending by score.
   */
  matchAgainst(
    query: Organization,
    candidates: Organization[],
  ): Promise<MatchRankResult[]> {
    return this.http.post<MatchRankResult[]>("/api/organizations/match", {
      body: { query, candidates },
    });
  }

  /**
   * `POST /api/organizations/review-queue/{id}/decision` — decide a
   * pending review item. Only `pending` items can be decided; the
   * service returns 422 for anything else (first writer wins).
   */
  decideReview(id: string, status: ReviewDecision): Promise<ReviewQueueItem> {
    return this.http.post<ReviewQueueItem>(
      `/api/organizations/review-queue/${encodeURIComponent(id)}/decision`,
      { body: { status } },
    );
  }

  /**
   * `POST /api/organizations/merge` — fold the duplicate into the
   * survivor: the survivor gains the duplicate's data (its name kept as
   * an alternate name) and the duplicate is soft-deleted. Destructive;
   * callers confirm first.
   *
   * @param request The two pids plus an optional reason for the history.
   * @returns The two pids and the survivor's post-merge payload.
   * @throws {@link ApiError} 422 when the pids are equal (self-merge),
   *   404 when either pid is unknown.
   */
  merge(request: MergeRequest): Promise<MergeResponse> {
    return this.http.post<MergeResponse>("/api/organizations/merge", {
      body: request,
    });
  }

  /**
   * `GET /api/organizations/merges/recent` — the merge history, newest
   * first (the service caps the response at 100 rows).
   */
  recentMerges(): Promise<MergeRecordRow[]> {
    return this.http.get<MergeRecordRow[]>("/api/organizations/merges/recent");
  }
}
