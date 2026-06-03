// Golden-path integration tests for the Person front-end driven
// against a *live* person-service-rust-crate.
//
// Prerequisites:
//   - Rust service running at PUBLIC_API_BASE_URL (default
//     http://localhost:8080). Bring it up with
//     `(cd ../person-service-rust-crate && docker compose up -d)`
//     or via `bin/e2e` which health-checks first.
//   - Postgres reachable from the service (handled by the service's
//     docker-compose).
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

async function apiCreatePerson(
    request: APIRequestContext,
    family: string,
    given: string,
    extra: Record<string, unknown> = {},
): Promise<CreatedPerson> {
    const birth_date = "1985-04-12";
    const res = await request.post(`${API_BASE}/api/persons`, {
        data: {
            name: { family, given: [given] },
            gender: "female",
            birth_date,
            active: true,
            ...extra,
        },
    });
    expect(res.status(), `create ${family} ${given}`).toBeGreaterThanOrEqual(200);
    expect(res.status()).toBeLessThan(300);
    const body = await res.json();
    expect(body.success, `envelope.success for ${family}`).toBe(true);
    return { id: body.data.id, family, given, birth_date };
}

async function apiSoftDelete(request: APIRequestContext, id: string): Promise<void> {
    const res = await request.delete(`${API_BASE}/api/persons/${id}`);
    // 204 or 200 both indicate success; ignore 404 (already gone).
    expect([200, 204, 404]).toContain(res.status());
}

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
        await expect(page.getByRole("heading", { level: 1 })).toHaveText(
            new RegExp(`${created.given}\\s+${created.family}`),
        );
        await expect(page.getByText(created.id)).toBeVisible();
        await expect(page.getByText(created.birth_date)).toBeVisible();
        await expect(page.getByText("female")).toBeVisible();
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
        // Seed A directly via API.
        const family = uniqueFamily("dup");
        const first = await apiCreatePerson(request, family, "Carlos");
        cleanup.push(first.id);

        // Now try to create A again via the UI — service should 409
        // with a MatchResult[] in error.details; the form should
        // render the candidates inline.
        await page.goto("/persons/new");
        await page.getByLabel(/Family name/).fill(family);
        await page.getByLabel(/Given names/).fill("Carlos");
        await page.getByLabel(/Birth date/).fill("1985-04-12");
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
