// Unit tests: the money formatter's masked/absent honesty, the API
// path map (contract mirror), and the 13-locale i18n parity pin.

import { describe, expect, it, vi } from "vitest";

import { money } from "../../src/lib/api/crm";
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
    expect(money(500000, "GBP", "en")).toBe("£5,000.00");
  });

  it("renders the masked/absent state as an em dash, never zero", () => {
    expect(money(null, "GBP", "en")).toBe("—");
    expect(money(undefined, "GBP", "en")).toBe("—");
    expect(money(500000, null, "en")).toBe("—");
  });
});

describe("i18n", () => {
  it("covers every key in every locale (parity)", () => {
    for (const locale of LOCALES) {
      const table = STRINGS_BY_LOCALE[locale];
      for (const key of STRING_KEYS) {
        expect(table[key], `${locale} missing ${key}`).toBeTruthy();
      }
      expect(Object.keys(table).sort()).toEqual([...STRING_KEYS].sort());
    }
  });

  it("translates with en fallback and flags RTL locales", () => {
    expect(translate("nav.contacts", "de")).toBe("Kontakte");
    expect(translate("nav.contacts", DEFAULT_LOCALE)).toBe("Contacts");
    expect(isRtl("ar")).toBe(true);
    expect(isRtl("ur")).toBe(true);
    expect(isRtl("en")).toBe(false);
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
    const crm = await import("../../src/lib/api/crm");
    await crm.listContacts();
    await crm.getContact("c1");
    await crm.listLeads();
    await crm.listDeals("p1");
    await crm.forecast();
    await crm.campaignFunnel("k1");
    await crm.listTickets();
    await crm.salesDashboard();
    await crm.leadStatus("l1", "contacted");
    await crm.staleDeals(7);
    await crm.followups();
    await crm.pipelineHygiene();
    await crm.executivePack();
    await crm.forecastTrends();
    await crm.slaRegister();
    await crm.dpo();
    await crm.followups("task");
    await crm.cadence(30);
    await crm.engagementWorkload();
    await crm.funnel("p1");
    await crm.membersHealth();
    await crm.consentByAccount();
    await crm.stakeholdersView();
    await crm.partnershipsRegister();
    await crm.membershipsView(90);
    expect(calls).toEqual([
      "/api/proxy/contacts",
      "/api/proxy/contacts/c1",
      "/api/proxy/leads",
      "/api/proxy/deals?pipeline=p1",
      "/api/proxy/forecast",
      "/api/proxy/campaigns/k1/funnel",
      "/api/proxy/tickets",
      "/api/proxy/dashboards/sales",
      "/api/proxy/leads/l1/status",
      "/api/proxy/insights/stale-deals?days=7",
      "/api/proxy/insights/followups",
      "/api/proxy/insights/pipeline-hygiene",
      "/api/proxy/insights/executive",
      "/api/proxy/insights/forecast-trends",
      "/api/proxy/insights/sla",
      "/api/proxy/insights/dpo",
      "/api/proxy/insights/followups?kind=task",
      "/api/proxy/insights/cadence?days=30",
      "/api/proxy/insights/engagement",
      "/api/proxy/insights/funnel?pipeline=p1",
      "/api/proxy/insights/members",
      "/api/proxy/insights/consent-by-account",
      "/api/proxy/insights/stakeholders",
      "/api/proxy/insights/partnerships",
      "/api/proxy/insights/memberships?days=90",
    ]);
    vi.unstubAllGlobals();
  });
});
