// Resource-bound wrapper over ApiClient for the care-pathway endpoints.

import { API_BASE_URL } from "$lib/config";
import { dryRunFormValue } from "$lib/bulk";
import type { BulkImportFormat } from "$lib/bulk";
import { ApiClient } from "./client";
import type { Page, PageRequest } from "./client";
import type {
  AuditEntry,
  BulkExportRequest,
  BulkJobAccepted,
  BulkJobView,
  CarePathway,
  CoverageInsight,
  DirectoryInsight,
  InstanceDetail,
  InstanceStatus,
  LanguagesInsight,
  MergeResult,
  PathwayEvent,
  PathwayInstance,
  PathwayRef,
  ProvidersInsight,
  ScoredRef,
  VariantsInsight,
} from "./types";

/**
 * Resource-bound facade over {@link ApiClient} for the care-pathway REST
 * endpoints. Each method maps to one endpoint, URL-encodes path/query
 * params, and returns the typed body (throwing {@link ApiError} on non-2xx
 * via the underlying client).
 */
export class CarePathwayRepository {
  /** @param http - The configured low-level JSON client to delegate to. */
  constructor(private readonly http: ApiClient) {}

  /**
   * Convenience constructor wiring an {@link ApiClient} to {@link API_BASE_URL}.
   *
   * @param fetchFn - Optional `fetch` to inject (e.g. SvelteKit `load`
   *   fetch or a test fake); defaults to the global `fetch`.
   * @returns A ready-to-use repository.
   */
  static withFetch(fetchFn?: typeof fetch): CarePathwayRepository {
    return new CarePathwayRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /**
   * List all care pathways.
   * @returns `{pid, name}` references for every pathway.
   */
  list(): Promise<PathwayRef[]> {
    return this.http.get<PathwayRef[]>("/api/care-pathways");
  }

  /**
   * `GET /api/care-pathways?limit=&offset=` — one page, with the total.
   * @param page The window; omitted values leave the service's defaults.
   */
  listPage(page: PageRequest = {}): Promise<Page<PathwayRef>> {
    return this.http.getPage<PathwayRef>("/api/care-pathways", page);
  }

  /**
   * Case-insensitive name search (server-side `ILIKE`, capped at 50 rows).
   * @param q - The free-text query; URL-encoded into the `q` param.
   * @returns Matching `{pid, name}` references.
   */
  /// Case-insensitive name search (`ILIKE`, cap 50). `q` is URL-encoded.
  search(q: string): Promise<PathwayRef[]> {
    return this.http.get<PathwayRef[]>(
      `/api/care-pathways/search?q=${encodeURIComponent(q)}`,
    );
  }

  /**
   * Fetch one full care-pathway record by pid.
   * @param pid - Persistent identifier; URL-encoded into the path.
   * @returns The full {@link CarePathway}.
   */
  get(pid: string): Promise<CarePathway> {
    return this.http.get<CarePathway>(
      `/api/care-pathways/${encodeURIComponent(pid)}`,
    );
  }

  /**
   * Create a new care pathway.
   * @param pathway - The record to create (only `name` is required).
   * @returns The new record's `{pid, name}` reference.
   */
  create(pathway: CarePathway): Promise<PathwayRef> {
    return this.http.post<PathwayRef>("/api/care-pathways", { body: pathway });
  }

  /**
   * Replace an existing care pathway.
   * @param pid - Persistent identifier; URL-encoded into the path.
   * @param pathway - The full replacement record.
   * @returns The updated record's `{pid, name}` reference.
   */
  update(pid: string, pathway: CarePathway): Promise<PathwayRef> {
    return this.http.put<PathwayRef>(
      `/api/care-pathways/${encodeURIComponent(pid)}`,
      {
        body: pathway,
      },
    );
  }

  /**
   * Soft-delete a care pathway. The service returns an empty body.
   * @param pid - Persistent identifier; URL-encoded into the path.
   */
  remove(pid: string): Promise<void> {
    return this.http.delete(`/api/care-pathways/${encodeURIComponent(pid)}`);
  }

  /**
   * Score a candidate record against the stored pathways for duplicates.
   * @param query - The record to match (typically the current detail record).
   * @returns Scored candidate references, ordered by the service.
   */
  /// Match a query against the stored care pathways.
  checkDuplicates(query: CarePathway): Promise<ScoredRef[]> {
    return this.http.post<ScoredRef[]>("/api/care-pathways/check-duplicates", {
      body: query,
    });
  }

  /**
   * Merge a duplicate into a survivor. Both pids travel in the request
   * body (not the URL); the survivor's refreshed record is returned.
   *
   * @param mainPid - The survivor (main) pid that absorbs the duplicate.
   * @param duplicatePid - The duplicate pid to fold in and soft-delete.
   * @param reason - Optional free-text reason recorded with the merge;
   *   omitted from the body entirely when undefined.
   * @returns The {@link MergeResult} with the survivor's refreshed record.
   * @throws {ApiError} `422` when the two pids are equal; `404` for an
   *   unknown pid.
   */
  /// Merge a duplicate into a survivor. Both pids travel in the body;
  /// returns the survivor's refreshed record. `422` for equal pids,
  /// `404` for an unknown pid.
  merge(
    mainPid: string,
    duplicatePid: string,
    reason?: string,
  ): Promise<MergeResult> {
    const body: { main_pid: string; duplicate_pid: string; reason?: string } = {
      main_pid: mainPid,
      duplicate_pid: duplicatePid,
    };
    // Only include `reason` when supplied, so the body is byte-stable
    // (the unit tests assert the exact serialized shape).
    if (reason !== undefined) {
      body.reason = reason;
    }
    return this.http.post<MergeResult>("/api/care-pathways/merge", { body });
  }

  /**
   * Fetch the audit trail for one pathway (service orders most-recent first).
   * @param pid - Persistent identifier; URL-encoded into the path.
   * @returns The audit-log entries for the pathway.
   */
  /// Audit trail for one care pathway, most-recent first. `pid` is
  /// URL-encoded.
  audit(pid: string): Promise<AuditEntry[]> {
    return this.http.get<AuditEntry[]>(
      `/api/care-pathways/${encodeURIComponent(pid)}/audit`,
    );
  }

  /**
   * Fetch recent system-wide CRUD/merge events from the in-memory stream.
   * The service returns them roughly oldest-first (highest `seq` last);
   * the caller re-sorts newest-first for display.
   *
   * @returns The recent {@link PathwayEvent} rows.
   */
  /// Recent system-wide CRUD/merge events from the service's in-memory
  /// stream. Returned roughly oldest-first (highest `seq` last); the UI
  /// sorts newest-first.
  recentEvents(): Promise<PathwayEvent[]> {
    return this.http.get<PathwayEvent[]>("/api/care-pathways/events/recent");
  }

  // -- Registry insight lenses (read-only) --------------------------------

  /** `GET /api/care-pathways/insights/directory`. */
  insightsDirectory(): Promise<DirectoryInsight> {
    return this.http.get<DirectoryInsight>(
      "/api/care-pathways/insights/directory",
    );
  }

  /** `GET /api/care-pathways/insights/coverage`. */
  insightsCoverage(): Promise<CoverageInsight> {
    return this.http.get<CoverageInsight>(
      "/api/care-pathways/insights/coverage",
    );
  }

  /** `GET /api/care-pathways/insights/variants`. */
  insightsVariants(): Promise<VariantsInsight> {
    return this.http.get<VariantsInsight>(
      "/api/care-pathways/insights/variants",
    );
  }

  /** `GET /api/care-pathways/insights/providers`. */
  insightsProviders(): Promise<ProvidersInsight> {
    return this.http.get<ProvidersInsight>(
      "/api/care-pathways/insights/providers",
    );
  }

  /** `GET /api/care-pathways/insights/languages`. */
  insightsLanguages(): Promise<LanguagesInsight> {
    return this.http.get<LanguagesInsight>(
      "/api/care-pathways/insights/languages",
    );
  }

  // -- Pathway instances --------------------------------------------------

  /**
   * `GET /api/care-pathways/{pathway}/instances` — the enrolments on one
   * pathway template.
   * @param pathwayPid - Template pid; URL-encoded into the path.
   */
  listInstances(pathwayPid: string): Promise<PathwayInstance[]> {
    return this.http.get<PathwayInstance[]>(
      `/api/care-pathways/${encodeURIComponent(pathwayPid)}/instances`,
    );
  }

  /**
   * `GET /api/instances/{pid}` — one instance plus its steps / team /
   * events / measures.
   * @param pid - Instance pid; URL-encoded into the path.
   */
  getInstance(pid: string): Promise<InstanceDetail> {
    return this.http.get<InstanceDetail>(
      `/api/instances/${encodeURIComponent(pid)}`,
    );
  }

  /**
   * `POST /api/instances/{pid}/status` — move an instance to a new
   * lifecycle status. The service's status machine refuses illegal
   * transitions with `422`.
   * @param pid - Instance pid; URL-encoded into the path.
   * @param to - The target {@link InstanceStatus}.
   * @returns The updated instance.
   */
  setInstanceStatus(pid: string, to: InstanceStatus): Promise<PathwayInstance> {
    return this.http.post<PathwayInstance>(
      `/api/instances/${encodeURIComponent(pid)}/status`,
      { body: { to } },
    );
  }

  /**
   * `GET /api/instances/caseload` — the derived operational caseload view
   * across all pathways (counts by status / urgency + due reviews).
   */
  caseload(): Promise<unknown> {
    return this.http.get<unknown>("/api/instances/caseload");
  }

  // -- Native bulk import / export (T-10) ---------------------------------

  /**
   * Submit a bulk import: upload the file as `multipart/form-data` and get
   * back the id of the enqueued job to poll.
   *
   * The body is a `FormData`, which {@link ApiClient} passes through
   * without JSON-serializing and without forcing a `content-type` (so
   * `fetch` can set the multipart boundary).
   *
   * @param file - The JSONL/CSV/TSV file the operator chose.
   * @param options - Format (defaults to `jsonl` server-side), dry-run
   *   preview flag, and an optional `Idempotency-Key` so a retried submit
   *   resolves to the original job (SEC-B9) rather than importing twice.
   * @throws {ApiError} 400 for a bad/unsupported upload, 413 when the file
   *   exceeds the service's 64 MiB import cap.
   */
  importPathways(
    file: File,
    options: {
      format?: BulkImportFormat;
      dryRun?: boolean;
      idempotencyKey?: string;
    } = {},
  ): Promise<BulkJobAccepted> {
    const form = new FormData();
    form.append("file", file, file.name);
    if (options.format) form.append("format", options.format);
    // Always sent, so the request states the operator's choice rather than
    // relying on an absent field meaning false.
    form.append("dry_run", dryRunFormValue(options.dryRun ?? false));
    return this.http.post<BulkJobAccepted>("/api/care-pathways/import", {
      body: form,
      headers: options.idempotencyKey
        ? { "idempotency-key": options.idempotencyKey }
        : undefined,
    });
  }

  /**
   * Submit a bulk export and get back the id of the enqueued job.
   *
   * @param request - Format, optional filter, and masking profile.
   * @param idempotencyKey - Optional key deduping a retried submit.
   * @throws {ApiError} 400 for an unsupported format/profile; 401 or 403
   *   when `masking_profile: "full"` is requested without elevated
   *   authorisation.
   */
  exportPathways(
    request: BulkExportRequest = {},
    idempotencyKey?: string,
  ): Promise<BulkJobAccepted> {
    return this.http.post<BulkJobAccepted>("/api/care-pathways/export", {
      body: request,
      headers: idempotencyKey
        ? { "idempotency-key": idempotencyKey }
        : undefined,
    });
  }

  /**
   * Status and row counts for one import job.
   *
   * @throws {ApiError} 404 once the job has passed its retention TTL, or
   *   if it belongs to another actor — the service deliberately does not
   *   distinguish the two.
   */
  getImportJob(id: string): Promise<BulkJobView> {
    return this.http.get<BulkJobView>(
      `/api/care-pathways/import/${encodeURIComponent(id)}`,
    );
  }

  /** Status and row counts for one export job. Same 404 semantics. */
  getExportJob(id: string): Promise<BulkJobView> {
    return this.http.get<BulkJobView>(
      `/api/care-pathways/export/${encodeURIComponent(id)}`,
    );
  }

  /**
   * Recent bulk jobs, newest first. Unlike person's/organization's own
   * bulk-jobs endpoint, this service's supports server-side `kind`/
   * `status` filtering directly, so this passes them through rather than
   * filtering the returned array client-side.
   *
   * @param options - Optional `kind`/`status` filter and a row `limit`
   *   (service default 50, capped at 500).
   */
  listBulkJobs(
    options: { kind?: string; status?: string; limit?: number } = {},
  ): Promise<BulkJobView[]> {
    const query = new URLSearchParams();
    if (options.kind) query.set("kind", options.kind);
    if (options.status) query.set("status", options.status);
    if (options.limit !== undefined) query.set("limit", String(options.limit));
    const suffix = query.toString();
    return this.http.get<BulkJobView[]>(
      `/api/care-pathways/bulk-jobs${suffix ? `?${suffix}` : ""}`,
    );
  }
}
