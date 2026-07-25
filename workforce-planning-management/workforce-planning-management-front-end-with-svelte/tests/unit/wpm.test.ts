// Unit tests: the money formatter's masked/absent honesty, the API
// path map (contract mirror), and the 13-locale i18n parity pin.

import { describe, expect, it, vi } from "vitest";

import { money } from "../../src/lib/api/wpm";
import {
  DEFAULT_LOCALE,
  LOCALES,
  STRING_KEYS,
  STRINGS_BY_LOCALE,
  isRtl,
  translate,
} from "../../src/lib/i18n.svelte";

describe("money", () => {
  it("formats minor units as locale currency", () => {
    expect(money(360000, "GBP", "en")).toBe("£3,600.00");
    expect(money(0, "GBP", "en")).toBe("£0.00");
  });

  it("renders the masked/absent state as an em dash, never zero", () => {
    expect(money(null, "GBP", "en")).toBe("—");
    expect(money(undefined, "GBP", "en")).toBe("—");
    expect(money(360000, null, "en")).toBe("—");
  });
});

describe("i18n", () => {
  it("covers every key in every locale (parity)", () => {
    for (const locale of LOCALES) {
      const table = STRINGS_BY_LOCALE[locale];
      for (const key of STRING_KEYS) {
        expect(table[key], `${locale} missing ${key}`).toBeTruthy();
      }
      // No stray keys either.
      expect(Object.keys(table).sort()).toEqual([...STRING_KEYS].sort());
    }
  });

  it("translates with en fallback and flags RTL locales", () => {
    expect(translate("nav.employees", "de")).toBe("Mitarbeiter");
    expect(translate("nav.employees", DEFAULT_LOCALE)).toBe("Employees");
    expect(isRtl("ar")).toBe(true);
    expect(isRtl("ur")).toBe(true);
    expect(isRtl("en")).toBe(false);
    expect(isRtl("es-MX")).toBe(false);
  });
});

describe("api path map", () => {
  it("calls the exact proxy paths the service mounts", async () => {
    const calls: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string | URL) => {
        calls.push(String(url));
        return new Response("[]", {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }),
    );
    const wpm = await import("../../src/lib/api/wpm");
    await wpm.listEmployees({ department: "engineering" });
    await wpm.getEmployee("p1");
    await wpm.orgChart("organization:abc");
    await wpm.listRequisitions("open");
    await wpm.listRuns();
    await wpm.runPayslips("r1");
    await wpm.benchmarkComparison("organization:abc");
    await wpm.listSkills();
    await wpm.skillsMatrix();
    await wpm.trainingAnalytics();
    await wpm.listPaths();
    await wpm.pathProgress("path1");
    await wpm.mentorshipOverview(30);
    await wpm.listWellbeingEntitlements();
    await wpm.employeeWellbeingPrompts("p1");
    await wpm.wellbeingUptake();
    await wpm.workingTime("engineering");
    await wpm.listPulseSurveys();
    await wpm.pulseResults("s1");
    await wpm.listAppraisals("p1");
    await wpm.getAppraisal("a1");
    await wpm.appraisalReport("a1");
    expect(calls).toEqual([
      "/api/proxy/employees?department=engineering",
      "/api/proxy/employees/p1",
      "/api/proxy/org-chart?organization=organization%3Aabc",
      "/api/proxy/requisitions?status=open",
      "/api/proxy/payroll-runs",
      "/api/proxy/payroll-runs/r1/payslips",
      "/api/proxy/benchmarks/comparison?organization=organization%3Aabc",
      "/api/proxy/skills",
      "/api/proxy/learning/skills-matrix",
      "/api/proxy/learning/training-analytics",
      "/api/proxy/learning-paths",
      "/api/proxy/learning-paths/path1/progress",
      "/api/proxy/learning/mentorship-overview?days=30",
      "/api/proxy/wellbeing-entitlements",
      "/api/proxy/employees/p1/wellbeing-prompts",
      "/api/proxy/wellbeing/uptake",
      "/api/proxy/workforce/working-time?department=engineering",
      "/api/proxy/pulse-surveys",
      "/api/proxy/pulse-surveys/s1/results",
      "/api/proxy/employees/p1/appraisals",
      "/api/proxy/appraisals/a1",
      "/api/proxy/appraisals/a1/report",
    ]);
    vi.unstubAllGlobals();
  });
});
