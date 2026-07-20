// Unit tests: the money formatter's masked/absent honesty, the API
// path map (contract mirror), and the 13-locale i18n parity pin.

import { describe, expect, it, vi } from "vitest";

import { money } from "../../src/lib/api/hcm";
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
    const hcm = await import("../../src/lib/api/hcm");
    await hcm.listEmployees({ department: "engineering" });
    await hcm.getEmployee("p1");
    await hcm.orgChart("organization:abc");
    await hcm.listRequisitions("open");
    await hcm.listRuns();
    await hcm.runPayslips("r1");
    await hcm.benchmarkComparison("organization:abc");
    await hcm.listSkills();
    await hcm.skillsMatrix();
    await hcm.trainingAnalytics();
    await hcm.listPaths();
    await hcm.pathProgress("path1");
    await hcm.mentorshipOverview(30);
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
    ]);
    vi.unstubAllGlobals();
  });
});
