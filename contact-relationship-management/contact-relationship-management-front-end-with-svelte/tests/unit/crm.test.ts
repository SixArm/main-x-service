// Unit tests: the money formatter's masked/absent honesty, the API
// path map (contract mirror), and the seven-locale i18n parity pin.

import { describe, expect, it, vi } from "vitest";

import { money } from "../../src/lib/api/crm";
import {
  DEFAULT_LOCALE,
  LOCALES,
  LOCALE_LABELS,
  STRING_KEYS,
  STRINGS_BY_LOCALE,
  i18n,
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

  it("supports exactly the seven family locales, sorted, no parentheses", () => {
    expect([...LOCALES]).toEqual([
      "ar-001",
      "cy-001",
      "de-de",
      "en-001",
      "es-001",
      "fr-001",
      "hi-001",
      "zh-cn",
    ]);
    expect(DEFAULT_LOCALE).toBe("en-001");
    for (const label of Object.values(LOCALE_LABELS)) {
      expect(label).not.toMatch(/[()]/);
    }
    expect(LOCALE_LABELS["zh-cn"]).toBe("中文 - 中国");
  });

  it("translates with en fallback and flags RTL locales", () => {
    expect(translate("nav.contacts", "es-001")).toBe("Contactos");
    expect(translate("nav.contacts", DEFAULT_LOCALE)).toBe("Contacts");
    expect(translate("nav.contacts", "xx" as never)).toBe("Contacts");
    expect(translate("share.copy_link", "en-001")).toBe("Copy Link");
    expect(isRtl("ar-001")).toBe(true);
    expect(isRtl("ar")).toBe(true);
    expect(isRtl("en-001")).toBe(false);
    expect(isRtl("zh-cn")).toBe(false);
  });

  it("normalises legacy and regional codes by primary language", () => {
    i18n.set("en_US");
    expect(i18n.locale).toBe("en-001");
    i18n.set("es-MX");
    expect(i18n.locale).toBe("es-001");
    i18n.set("zh");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("zh-CN");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("de-DE");
    expect(i18n.locale).toBe("de-de");
    i18n.set("ja");
    expect(i18n.locale).toBe("en-001");
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

describe("tour", () => {
  const TOUR_KEYS = [
    "nav.tour",
    "tour.head",
    "tour.intro",
    ...[1, 2, 3, 4, 5, 6].flatMap((n) => [
      `tour.s${n}.title`,
      `tour.s${n}.summary`,
      ...[1, 2, 3, 4].map((k) => `tour.s${n}.step.${k}`),
    ]),
  ];

  it("has six workflow sections of four steps in every locale", () => {
    for (const locale of LOCALES) {
      for (const key of TOUR_KEYS) {
        expect(
          STRINGS_BY_LOCALE[locale][
            key as keyof (typeof STRINGS_BY_LOCALE)["en-001"]
          ],
          `${locale} missing ${key}`,
        ).toBeTruthy();
      }
    }
  });

  it("is public: the root layout gate exempts /tour", async () => {
    const { load } = await import("../../src/routes/+layout.server");
    const run = (path: string) =>
      load({
        locals: { sessionId: null },
        url: new URL(`http://localhost${path}`),
      } as unknown as Parameters<typeof load>[0]);
    expect(run("/tour")).toEqual({ signedIn: false });
    expect(() => run("/contacts")).toThrow();
  });
});
