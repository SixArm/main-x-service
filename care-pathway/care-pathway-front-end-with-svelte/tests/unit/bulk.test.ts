// Unit tests for the native bulk import/export surface (T-10): the pure
// client-side rules in `$lib/bulk`, and that each CarePathwayRepository
// method hits the right endpoint/verb/headers with the right body
// encoding. Uses an injected fake fetch (no network).
import { describe, expect, it } from "vitest";
import { ApiClient, isFormDataBody } from "../../src/lib/api/client";
import { CarePathwayRepository } from "../../src/lib/api/care-pathways";
import {
  BULK_FORMATS,
  BULK_IMPORT_FORMATS,
  dryRunFormValue,
  isTerminalStatus,
  progressPercent,
} from "../../src/lib/bulk";
import type { BulkJobView } from "../../src/lib/api/types";

// Cast a plain async function to the `fetch` type so it can be injected.
function mockFetch(
  impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
) {
  return impl as unknown as typeof fetch;
}

// Build a JSON Response with the given body/status for the fake fetch.
// The service returns RAW JSON (no {success,data,error} envelope).
function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

// Reusable fixture job in mid-flight.
const sampleJob: BulkJobView = {
  id: "j1",
  kind: "import",
  entity: "care_pathway",
  format: "jsonl",
  status: "running",
  rows_total: 10,
  rows_processed: 4,
  rows_created: 3,
  rows_upserted: 1,
  rows_to_review: 0,
  rows_errored: 0,
  download_url: null,
  errors_url: null,
};

describe("bulk client-side rules", () => {
  // Pins: polling stops on exactly the three states the worker leaves a
  // job in, and `completed_with_errors` is one of them (treating it as
  // non-terminal would poll forever).
  it("treats only the three finished states as terminal", () => {
    expect(isTerminalStatus("completed")).toBe(true);
    expect(isTerminalStatus("completed_with_errors")).toBe(true);
    expect(isTerminalStatus("failed")).toBe(true);
    expect(isTerminalStatus("queued")).toBe(false);
    expect(isTerminalStatus("running")).toBe(false);
  });

  // Pins: an unrecognised status from a newer service must not stop the
  // poll — the UI would silently freeze on a job still in flight.
  it("does not treat an unknown status as terminal", () => {
    expect(isTerminalStatus("paused")).toBe(false);
    expect(isTerminalStatus("")).toBe(false);
  });

  // Pins: the dry-run encoding matches the Rust handler's truthy token
  // set (`1` | `true` | `yes` | `on`, trimmed); anything else is false.
  it("encodes dry-run as a token the service reads as true", () => {
    expect(dryRunFormValue(true)).toBe("true");
    expect(["1", "true", "yes", "on"]).toContain(dryRunFormValue(true));
    expect(["1", "true", "yes", "on"]).not.toContain(dryRunFormValue(false));
  });

  // Pins: an uncounted total (an early poll) is indeterminate, not 0%.
  it("reports progress only once the total is known", () => {
    expect(progressPercent(4, 10)).toBe(40);
    expect(progressPercent(0, 10)).toBe(0);
    expect(progressPercent(4, null)).toBeNull();
    expect(progressPercent(4, 0)).toBeNull();
  });

  // Pins: an over-counting worker cannot render a >100% bar.
  it("clamps progress to 0–100", () => {
    expect(progressPercent(15, 10)).toBe(100);
    expect(progressPercent(-5, 10)).toBe(0);
  });

  // Pins: care-pathway's native formats are symmetric (no Parquet-style
  // export-only carve-out), unlike person's — every export format is
  // also importable.
  it("offers the same formats for import and export", () => {
    expect([...BULK_IMPORT_FORMATS]).toEqual([...BULK_FORMATS]);
    expect([...BULK_FORMATS]).toEqual(["jsonl", "csv", "tsv"]);
  });
});

describe("ApiClient FormData handling", () => {
  // Pins: the type guard answers "no" for the JSON bodies every other
  // repository method sends, so their encoding is untouched.
  it("recognises FormData and nothing else", () => {
    expect(isFormDataBody(new FormData())).toBe(true);
    expect(isFormDataBody({ a: 1 })).toBe(false);
    expect(isFormDataBody("a=1")).toBe(false);
    expect(isFormDataBody(undefined)).toBe(false);
  });

  // Pins: a JSON body still gets serialized and still carries the JSON
  // content-type — the multipart change must not alter existing calls.
  it("still JSON-serializes a plain object body", async () => {
    let capturedBody: unknown;
    let capturedType: string | undefined;
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (_input, init) => {
        capturedBody = init?.body;
        capturedType = (init?.headers as Record<string, string>)[
          "content-type"
        ];
        return jsonResponse({});
      }),
    });
    await client.post("/api/care-pathways", { body: { a: 1 } });
    expect(typeof capturedBody).toBe("string");
    expect(JSON.parse(capturedBody as string)).toEqual({ a: 1 });
    expect(capturedType).toBe("application/json");
  });
});

describe("CarePathwayRepository bulk methods", () => {
  // Pins the multipart contract end-to-end: a FormData body (not JSON),
  // no forced content-type (so `fetch` can set the boundary), the file
  // and format fields, the dry-run token, and the Idempotency-Key header.
  it("POSTs a FormData import with the idempotency key", async () => {
    let capturedUrl = "";
    let capturedBody: unknown;
    let capturedHeaders: Record<string, string> = {};
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input, init) => {
        capturedUrl = String(input);
        capturedBody = init?.body;
        capturedHeaders = init?.headers as Record<string, string>;
        return jsonResponse({ job_id: "j1" });
      }),
    });
    const repo = new CarePathwayRepository(client);
    const file = new File(['{"name":"Stroke pathway"}'], "pathways.jsonl", {
      type: "application/jsonl",
    });

    const result = await repo.importPathways(file, {
      format: "jsonl",
      dryRun: true,
      idempotencyKey: "key-123",
    });

    expect(capturedUrl).toContain("/api/care-pathways/import");
    // The body must be the FormData itself, never a JSON string.
    expect(capturedBody).toBeInstanceOf(FormData);
    const form = capturedBody as FormData;
    expect(form.get("format")).toBe("jsonl");
    expect(form.get("dry_run")).toBe("true");
    expect(form.get("file")).toBeInstanceOf(File);
    // `fetch` must own the content-type here — a leftover
    // application/json would make the server answer 400 bad_multipart.
    expect(
      Object.keys(capturedHeaders).some(
        (k) => k.toLowerCase() === "content-type",
      ),
    ).toBe(false);
    expect(capturedHeaders["idempotency-key"]).toBe("key-123");
    expect(result.job_id).toBe("j1");
  });

  // Pins: an unchecked dry-run box still sends the field, with a value
  // the service reads as false, and no header is invented without a key.
  it("sends dry_run=false and no idempotency header by default", async () => {
    let capturedBody: unknown;
    let capturedHeaders: Record<string, string> = {};
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (_input, init) => {
        capturedBody = init?.body;
        capturedHeaders = init?.headers as Record<string, string>;
        return jsonResponse({ job_id: "j2" });
      }),
    });
    const repo = new CarePathwayRepository(client);
    await repo.importPathways(new File(["x"], "pathways.csv"), {});
    expect((capturedBody as FormData).get("dry_run")).toBe("false");
    expect(capturedHeaders["idempotency-key"]).toBeUndefined();
  });

  // Pins: export is a JSON POST carrying the filter + masking profile.
  it("POSTs a JSON export request", async () => {
    let capturedUrl = "";
    let capturedBody = "";
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input, init) => {
        capturedUrl = String(input);
        capturedBody = init?.body as string;
        return jsonResponse({ job_id: "j3" });
      }),
    });
    const repo = new CarePathwayRepository(client);
    const result = await repo.exportPathways({
      format: "csv",
      q: "stroke",
      limit: 100,
      masking_profile: "masked",
    });
    expect(capturedUrl).toContain("/api/care-pathways/export");
    expect(JSON.parse(capturedBody)).toEqual({
      format: "csv",
      q: "stroke",
      limit: 100,
      masking_profile: "masked",
    });
    expect(result.job_id).toBe("j3");
  });

  // Pins: the two status endpoints are distinct paths, both GET.
  it("GETs the import and export job status endpoints", async () => {
    const urls: string[] = [];
    const methods: string[] = [];
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input, init) => {
        urls.push(String(input));
        methods.push(init?.method ?? "");
        return jsonResponse(sampleJob);
      }),
    });
    const repo = new CarePathwayRepository(client);
    const imported = await repo.getImportJob("j1");
    await repo.getExportJob("j9");
    expect(urls[0]).toContain("/api/care-pathways/import/j1");
    expect(urls[1]).toContain("/api/care-pathways/export/j9");
    expect(methods).toEqual(["GET", "GET"]);
    expect(imported.rows_processed).toBe(4);
  });

  // Pins: a job past its retention TTL (or another actor's) answers 404,
  // which must surface as a catchable ApiError rather than a crash.
  it("surfaces a 404 job status as a not-found ApiError", async () => {
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async () => jsonResponse({ error: "not_found" }, 404)),
    });
    const repo = new CarePathwayRepository(client);
    await expect(repo.getImportJob("j1")).rejects.toMatchObject({
      status: 404,
    });
  });

  // Pins: unlike person's bulk-jobs endpoint (limit-only), this
  // service's list supports server-side `kind`/`status` filtering, and
  // an omitted filter/limit sends no corresponding query param.
  it("GETs the bulk-jobs list with kind/status/limit params", async () => {
    const urls: string[] = [];
    const client = new ApiClient({
      baseUrl: "http://test",
      fetch: mockFetch(async (input) => {
        urls.push(String(input));
        return jsonResponse([sampleJob]);
      }),
    });
    const repo = new CarePathwayRepository(client);
    const result = await repo.listBulkJobs({
      kind: "import",
      status: "completed",
      limit: 25,
    });
    await repo.listBulkJobs();
    expect(urls[0]).toContain("/api/care-pathways/bulk-jobs?");
    expect(urls[0]).toContain("kind=import");
    expect(urls[0]).toContain("status=completed");
    expect(urls[0]).toContain("limit=25");
    expect(urls[1]).toBe("http://test/api/care-pathways/bulk-jobs");
    expect(result).toHaveLength(1);
  });
});
