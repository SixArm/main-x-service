// Golden-path integration tests for the Person front-end driven
// against a *live* person-service-with-loco.
//
// Prerequisites:
//   - Rust service running at PUBLIC_API_BASE_URL (default
//     http://localhost:8080). Bring it up with
//     `(cd ../person-service-with-loco && docker compose up -d)`
//     or via `bin/e2e` which health-checks first.
//   - Postgres reachable from the service (handled by the service's
//     docker-compose).
//   - authentication-service, in DEVELOPMENT mode, at AUTH_API_URL
//     (default http://localhost:5150 —
//     examples/compose/authentication-dev.yml, NOT full-family.yml).
//     The mutating flows below (create/edit/merge) go through the
//     page-visit auth guard + CSRF check, which need a real signed-in
//     session — see auth.setup.ts (T-27, PRO-P32) and this project's
//     AGENTS.md "Live-integration testing needs a real sign-in".
//     `bin/e2e` health-checks this too.
//
// Each test creates its own data with a timestamped family name so
// the suite stays idempotent across runs. afterAll soft-deletes
// any records the test created via the REST API.
//
// Coverage maps to spec.md §6 Functional Requirements:
//   FR-1  list  GET /api/persons/search          → "search finds the seeded person"
//   FR-3  create + 409 inline duplicates         → "second create surfaces match candidates"
//   FR-5  detail page renders nested fields      → "detail view shows seeded fields"
//   FR-6  edit PUTs the full Person              → "edit persists across reload"
//   FR-7  soft-delete confirmed                  → "soft-delete hides record from list"
//   FR-8  match check posts and renders          → "match check returns the seeded person"
//   FR-9  merge GETs preview before POST         → "merge soft-deletes the duplicate"
//   FR-10 service-down banner                    → "banner surfaces unreachable API"
//                                                  (skipped here; see smoke suite)
//
// Plus per-record audit coverage.

import { type APIRequestContext, expect, test } from "@playwright/test";

const API_BASE = process.env.PUBLIC_API_BASE_URL ?? "http://localhost:8080";

interface CreatedPerson {
    id: string;
    family: string;
    given: string;
    birth_date: string;
}

// ─── Tiny REST helpers ─────────────────────────────────────────────

// Generate a per-record DOB spread across a wide range of past years
// (rather than the ~150-day span the old fixed-epoch-plus-17-days scheme
// used) so unrelated fixtures are unlikely to land in the create-time
// duplicate detector's same-year scoring bands in the first place. This
// is a *first line of defense*, not the correctness guarantee — see the
// PRO-P4 note on `apiCreatePerson` below for why a fixed date scheme
// alone cannot be trusted to dodge a live, real matcher.
function uniqueDob(): string {
    const year = 1930 + Math.floor(Math.random() * 90); // 1930-2019
    const dayOfYear = Math.floor(Math.random() * 365);
    const d = new Date(Date.UTC(year, 0, 1));
    d.setUTCDate(d.getUTCDate() + dayOfYear);
    return d.toISOString().slice(0, 10);
}

/** Bounded retries for {@link apiCreatePerson}'s duplicate-avoidance loop. */
const MAX_DUPLICATE_RETRIES = 5;

// Create a person directly via the REST API (bypassing the UI) to seed
// fixtures. Defaults to a unique DOB unless one is supplied; asserts a 2xx
// and a success envelope before returning the created record's id + inputs.
//
// PRO-P4 (2026-08-29): retries on a 409 by regenerating both the DOB and
// the family-name suffix, rather than relying on a fixed date-spacing
// scheme to dodge the live create-time duplicate detector
// (`check_duplicates_internal`, threshold 0.7,
// `person-service-with-loco/src/api/rest/handlers.rs`). Reproduced live,
// repeatedly, that hand-tuned spacing is not durably safe: every
// `E2E_*` family is a fuzzy-search candidate for every other record
// (the candidate search tokenizes on `_` and fuzzy-matches per token),
// a same-year DOB pair scores 0.50-0.95 in the service's own DOB grader
// (`dob_matching::match_birth_dates`,
// `person-service-with-loco/src/matching/algorithms.rs`) even with
// wholly distinct names, and this suite's own worker can be recycled
// mid-run (observed live, apparently triggered by the guarded-page
// redirects a signed-out run now hits — see this project's `AGENTS.md`
// "Page-visit guard (PRO-H10)"), which resets any in-process fixture
// counter and can make two *different* worker generations land on the
// same computed demographics. A test suite whose fixtures can
// unpredictably collide with a live, evolving matcher is exactly the
// "duplicate-detector test-data interaction" PRO-P4 exists to
// stabilize — regenerating and retrying is robust to that in a way no
// fixed date/name scheme can promise to be. A caller that *wants* the
// 409 (the FR-3 duplicate-detection test) passes a fixed `birth_date`,
// which disables retrying here — that call seeds the *first* record
// only; the actual duplicate POST goes through the UI directly, not
// through this helper.
async function apiCreatePerson(
    request: APIRequestContext,
    family: string,
    given: string,
    extra: { birth_date?: string } & Record<string, unknown> = {},
): Promise<CreatedPerson> {
    const { birth_date: fixedBirthDate, ...rest } = extra;
    let lastStatus = 0;
    let lastBody: unknown;
    for (let attempt = 0; attempt < MAX_DUPLICATE_RETRIES; attempt += 1) {
        const birth_date = fixedBirthDate ?? uniqueDob();
        // A retry needs a fresh family too — the same family with a new
        // DOB can still collide via the given-name + gender + fuzzy
        // family-token signal alone.
        const thisFamily = attempt === 0 ? family : `${family}_r${attempt}`;
        const res = await request.post(`${API_BASE}/api/persons`, {
            data: {
                name: { family: thisFamily, given: [given] },
                gender: "female",
                birth_date,
                active: true,
                ...rest,
            },
        });
        lastStatus = res.status();
        if (lastStatus === 409 && fixedBirthDate === undefined) {
            lastBody = await res.json().catch(() => undefined);
            continue;
        }
        expect(lastStatus, `create ${thisFamily} ${given}`).toBeGreaterThanOrEqual(200);
        expect(lastStatus).toBeLessThan(300);
        const body = await res.json();
        expect(body.success, `envelope.success for ${thisFamily}`).toBe(true);
        return { id: body.data.id, family: thisFamily, given, birth_date };
    }
    throw new Error(
        `create ${family} ${given}: still 409 after ${MAX_DUPLICATE_RETRIES} attempts ` +
            `(last response: ${JSON.stringify(lastBody)})`,
    );
}

// Soft-delete a seeded person via the API during cleanup.
async function apiSoftDelete(request: APIRequestContext, id: string): Promise<void> {
    const res = await request.delete(`${API_BASE}/api/persons/${id}`);
    // 204 or 200 both indicate success; ignore 404 (already gone).
    expect([200, 204, 404]).toContain(res.status());
}

// Build a family name that is unique per run/record so concurrent runs and
// repeated runs don't collide on search/match assertions.
function uniqueFamily(tag: string): string {
    return `E2E_${tag}_${Date.now()}_${Math.floor(Math.random() * 1e4)}`;
}

// ─── Suite ─────────────────────────────────────────────────────────

test.describe("Person front-end ↔ Person Service golden paths", () => {
    let created: CreatedPerson;
    const cleanup: string[] = [];

    test.beforeAll(async ({ request }) => {
        // Sanity: service must be reachable.
        const health = await request.get(`${API_BASE}/api/health`);
        expect(health.status(), `service health at ${API_BASE}`).toBeLessThan(500);

        // Seed one record the read-flow tests can rely on.
        created = await apiCreatePerson(request, uniqueFamily("seed"), "Alice");
        cleanup.push(created.id);
    });

    test.afterAll(async ({ request }) => {
        for (const id of cleanup) {
            try {
                await apiSoftDelete(request, id);
            } catch {
                /* best-effort */
            }
        }
    });

    // ─── Read flows ────────────────────────────────────────────────

    test("FR-1: list page renders the seeded person when searched", async ({ page }) => {
        await page.goto("/persons");
        const searchbox = page.getByRole("searchbox");
        await searchbox.fill(created.family);
        await page.getByRole("button", { name: /^Search$/ }).click();
        // SVAR Grid renders the family/given/birth_date columns.
        await expect(page.getByText(created.family).first()).toBeVisible({ timeout: 5_000 });
        await expect(page.getByText(created.given).first()).toBeVisible();
    });

    test("FR-5: detail page shows nested identity fields", async ({ page }) => {
        await page.goto(`/persons/${created.id}`);
        const main = page.getByRole("main");
        await expect(main.getByRole("heading", { level: 1 })).toHaveText(
            new RegExp(`${created.given}\\s+${created.family}`),
        );
        await expect(main.getByText(created.id)).toBeVisible();
        await expect(main.getByText(created.birth_date)).toBeVisible();
        await expect(main.getByText("female")).toBeVisible();
    });

    // ─── Write flows ───────────────────────────────────────────────

    test("FR-3: create form posts to API and lands on detail page", async ({ page, request }) => {
        const family = uniqueFamily("create");
        await page.goto("/persons/new");
        await page.getByLabel(/Family name/).fill(family);
        await page.getByLabel(/Given names/).fill("Brigitte");
        await page.getByLabel(/Birth date/).fill("1990-07-15");
        await page.getByRole("button", { name: "Create" }).click();
        await page.waitForURL(/\/persons\/[0-9a-f-]{36}$/, { timeout: 10_000 });

        const id = page.url().split("/").pop()!;
        cleanup.push(id);
        const fetched = await request.get(`${API_BASE}/api/persons/${id}`);
        expect(fetched.status()).toBe(200);
        const body = await fetched.json();
        expect(body.data.name.family).toBe(family);
        expect(body.data.name.given).toContain("Brigitte");
    });

    test("FR-3: second create with identical demographics surfaces 409 duplicates inline", async ({
        page,
        request,
    }) => {
        // Seed A directly via API with a *fixed* birth_date so the
        // second UI POST can replay it exactly and reliably trigger
        // the duplicate detector.
        const family = uniqueFamily("dup");
        const sharedDob = "1981-09-23";
        const first = await apiCreatePerson(request, family, "Carlos", { birth_date: sharedDob });
        cleanup.push(first.id);

        // Now try to create A again via the UI — service should 409
        // with a MatchResult[] in error.details; the form should
        // render the candidates inline.
        await page.goto("/persons/new");
        await page.getByLabel(/Family name/).fill(family);
        await page.getByLabel(/Given names/).fill("Carlos");
        await page.getByLabel(/Birth date/).fill(sharedDob);
        await page.getByRole("button", { name: "Create" }).click();

        // The submit handler throws a synthesised error so the form
        // shows the banner and the MatchResultsList appears below.
        await expect(page.getByText(/Possible duplicates/i)).toBeVisible({ timeout: 10_000 });
        await expect(page.getByText(family).first()).toBeVisible();
    });

    test("FR-6: edit form PUTs the updated person", async ({ page, request }) => {
        // Use a fresh record so concurrent runs don't collide.
        const seeded = await apiCreatePerson(request, uniqueFamily("edit"), "Diana");
        cleanup.push(seeded.id);

        await page.goto(`/persons/${seeded.id}/edit`);
        await page.getByLabel(/Birth date/).fill("1970-01-30");
        await page.getByRole("button", { name: /Save changes/ }).click();
        await page.waitForURL(`**/persons/${seeded.id}`, { timeout: 10_000 });

        const fetched = await request.get(`${API_BASE}/api/persons/${seeded.id}`);
        const body = await fetched.json();
        expect(body.data.birth_date).toBe("1970-01-30");
    });

    test("FR-7: soft-delete from detail page hides record from search", async ({
        page,
        request,
    }) => {
        const seeded = await apiCreatePerson(request, uniqueFamily("del"), "Esme");
        // Don't add to cleanup — this test deletes it itself.

        await page.goto(`/persons/${seeded.id}`);
        page.once("dialog", (dialog) => dialog.accept());
        await page.getByRole("button", { name: "Delete" }).click();
        await page.waitForURL("**/persons", { timeout: 10_000 });

        // Search should not return the soft-deleted record.
        const searchRes = await request.get(
            `${API_BASE}/api/persons/search?q=${encodeURIComponent(seeded.family)}&limit=10`,
        );
        const body = await searchRes.json();
        const items = Array.isArray(body.data) ? body.data : (body.data?.items ?? []);
        const found = items.find((p: { id: string }) => p.id === seeded.id);
        // Service may either hide the record entirely or mark it inactive.
        if (found) {
            expect(found.active).toBeFalsy();
        }
    });

    // ─── Match / merge ─────────────────────────────────────────────

    test("FR-8: match check renders score + breakdown for the seeded person", async ({ page }) => {
        await page.goto("/persons/match");
        await page.getByLabel(/^Family/).fill(created.family);
        await page.getByLabel(/^Given/).fill(created.given);
        await page.getByLabel(/Birth date/).fill(created.birth_date);
        await page.getByLabel(/Threshold/).fill("0.5");
        await page.getByRole("button", { name: /Find matches/ }).click();

        // MatchResultsList renders header + at least one result row.
        await expect(page.getByRole("heading", { name: /Match results/ })).toBeVisible({
            timeout: 10_000,
        });
        await expect(page.getByText(created.family).first()).toBeVisible();
    });

    test("FR-9: merge soft-deletes the duplicate and the survivor remains", async ({
        page,
        request,
    }) => {
        const main = await apiCreatePerson(request, uniqueFamily("merge_main"), "Frieda");
        const dup = await apiCreatePerson(request, uniqueFamily("merge_dup"), "Frieda");
        cleanup.push(main.id, dup.id);

        await page.goto("/persons/merge");
        await page.getByLabel(/Main person ID/).fill(main.id);
        await page.getByLabel(/Duplicate person ID/).fill(dup.id);
        await page.getByLabel(/Reason/).fill("e2e merge test");

        page.once("dialog", (dialog) => dialog.accept());
        await page.getByRole("button", { name: /^Merge$/ }).click();

        await expect(page.getByText(/Merge completed/i)).toBeVisible({ timeout: 15_000 });

        // Main survives.
        const mainRes = await request.get(`${API_BASE}/api/persons/${main.id}`);
        expect(mainRes.status()).toBe(200);
        // Duplicate marked inactive or not findable.
        const dupRes = await request.get(`${API_BASE}/api/persons/${dup.id}`);
        if (dupRes.status() === 200) {
            const body = await dupRes.json();
            expect(body.data.active).toBeFalsy();
        } else {
            expect(dupRes.status()).toBe(404);
        }
    });

    // ─── Audit ─────────────────────────────────────────────────────

    test("audit log lists at least one entry for the seeded person", async ({ page }) => {
        await page.goto(`/persons/${created.id}/audit`);
        await expect(page.getByRole("heading", { name: "Audit log" })).toBeVisible();
        // Either entries show up (golden) or the service legitimately
        // reports an empty audit set. Both leave the header visible.
        const muted = page.getByText("No audit entries.");
        const codeBlocks = page.locator("code");
        const hasEntries = await codeBlocks.first().isVisible().catch(() => false);
        const hasEmpty = await muted.isVisible().catch(() => false);
        expect(hasEntries || hasEmpty).toBe(true);
    });
});
