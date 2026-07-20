// PpmClient path mapping + money formatting (the client is the single
// source of PPM endpoint paths, so a wrong path fails here first).

import { describe, expect, it, vi } from "vitest";
import { ApiClient } from "$lib/api/client";
import { PpmClient, money } from "$lib/api/ppm";

/** A PpmClient whose fetch records every (url, method) pair. */
function recordingClient(): { ppm: PpmClient; calls: { url: string; method: string }[] } {
  const calls: { url: string; method: string }[] = [];
  const fetchFn = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    calls.push({ url: String(input), method: init?.method ?? "GET" });
    return new Response("{}", {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }) as unknown as typeof fetch;
  const ppm = new PpmClient(new ApiClient({ baseUrl: "http://svc", fetch: fetchFn }));
  return { ppm, calls };
}

describe("PpmClient paths", () => {
  it("maps the catalogue endpoints", async () => {
    const { ppm, calls } = recordingClient();
    await ppm.dashboard();
    await ppm.listProposals("draft");
    await ppm.proposalAction("p1", "approve");
    await ppm.promoteProposal("p1");
    await ppm.proposalDuplicates("p1");
    await ppm.convertIdea("i1", "projects");
    await ppm.governance("projects", "w1");
    await ppm.reviewGate("projects", "w1", { gate: "g0_concept", decision: "approved" });
    await ppm.escalateRisk("programs", "w2", "r1");
    await ppm.recordActual("portfolios", "w3", "l1", 500);
    await ppm.realizeBenefit("products", "w4", "b1", { amount_minor: 1 });
    await ppm.schedule("w3");
    await ppm.capacity("2026-07-01", "2026-07-31");
    await ppm.evaluateScenario("s1");
    await ppm.commitScenario("s1");
    await ppm.alignment("o1");
    await ppm.linkObjective("projects", "w1", "o1", 4);
    await ppm.runReport("rep1");
    await ppm.executiveHealth();
    await ppm.executiveDecisions(10);
    await ppm.executiveBenefits();
    await ppm.financialVariance();
    await ppm.financialExposure();
    await ppm.technologyDependencyRisk();
    await ppm.technologyRadar();
    await ppm.executiveAlignment();
    await ppm.technologyDebt();
    await ppm.technologyFlow(3);
    await ppm.compareScenarios("s1", "s2");
    await ppm.boardPack();
    await ppm.boardInvestments();
    await ppm.takeSnapshot();
    await ppm.boardTrends();
    await ppm.auditorTrail({ action: "merged" });
    await ppm.auditorFindings();
    await ppm.complianceRegister();
    await ppm.complianceFindings();
    await ppm.riskHeatmap();
    await ppm.securityRegister();
    await ppm.regulatorExtract();

    expect(calls.map((call) => `${call.method} ${call.url}`)).toEqual([
      "GET http://svc/api/at-a-glance",
      "GET http://svc/api/proposals?status=draft",
      "POST http://svc/api/proposals/p1/approve",
      "POST http://svc/api/proposals/p1/promote",
      "GET http://svc/api/proposals/p1/duplicates",
      "POST http://svc/api/ideas/i1/convert",
      "GET http://svc/api/projects/w1/governance",
      "POST http://svc/api/projects/w1/gate-reviews",
      "POST http://svc/api/programs/w2/risks/r1/escalate",
      "POST http://svc/api/portfolios/w3/budget-lines/l1/actual",
      "POST http://svc/api/products/w4/benefits/b1/realize",
      "GET http://svc/api/portfolios/w3/schedule",
      "GET http://svc/api/capacity?from=2026-07-01&to=2026-07-31",
      "GET http://svc/api/scenarios/s1/evaluate",
      "POST http://svc/api/scenarios/s1/commit",
      "GET http://svc/api/objectives/o1/alignment",
      "POST http://svc/api/projects/w1/objectives",
      "GET http://svc/api/reports/rep1/run",
      "GET http://svc/api/executive/health",
      "GET http://svc/api/executive/decisions?limit=10",
      "GET http://svc/api/executive/benefits",
      "GET http://svc/api/financials/variance",
      "GET http://svc/api/financials/exposure",
      "GET http://svc/api/technology/dependency-risk",
      "GET http://svc/api/technology/radar",
      "GET http://svc/api/executive/alignment",
      "GET http://svc/api/technology/debt",
      "GET http://svc/api/technology/flow?months=3",
      "GET http://svc/api/scenarios/compare?a=s1&b=s2",
      "GET http://svc/api/board/pack",
      "GET http://svc/api/board/investments",
      "POST http://svc/api/board/snapshots",
      "GET http://svc/api/board/trends",
      "GET http://svc/api/auditor/trail?action=merged",
      "GET http://svc/api/auditor/findings",
      "GET http://svc/api/compliance/register",
      "GET http://svc/api/compliance/findings",
      "GET http://svc/api/risk/heatmap",
      "GET http://svc/api/security/register",
      "GET http://svc/api/regulator/extract",
    ]);
  });

  it("builds the CSV download URL against the configured base", () => {
    const { ppm } = recordingClient();
    // reportCsvUrl uses the app-wide API_BASE_URL (the BFF proxy).
    expect(ppm.reportCsvUrl("rep1")).toContain("/api/reports/rep1/run?format=csv");
  });
});

describe("money", () => {
  it("formats minor units as grouped major units", () => {
    expect(money(123_450, "GBP")).toBe("1,234.50 GBP");
    expect(money(5, "EUR")).toBe("0.05 EUR");
    expect(money(-99, "GBP")).toBe("-0.99 GBP");
    expect(money(100_00)).toBe("100.00");
  });
});
