// BedCard state × flags matrix (PF-T16): the card is the whiteboard's
// load-bearing unit, so every state and every chip is pinned here.

import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import BedCard from "$lib/components/BedCard.svelte";
import type { BedCard as Card } from "$lib/api/types";

/** A card factory: an available, empty, unremarkable bed. */
function card(overrides: Partial<Card> = {}): Card {
  return {
    bed_pid: "bed-1",
    bay_name: "Bay A",
    number: "W7-A-1",
    state: "available",
    state_since: "2026-07-18T08:00:00Z",
    closure_reason: null,
    deep_clean_required: false,
    side_room: false,
    stay_pid: null,
    display_name: null,
    named_nurse_ref: null,
    consultant_ref: null,
    edd: null,
    edd_missing: false,
    edd_overdue: false,
    ccd_met: false,
    discharge_pathway: null,
    discharge_ready: false,
    dtoc: false,
    senior_review_today: false,
    red_green_today: null,
    infection: [],
    alerts: [],
    ...overrides,
  };
}

/** An occupied card with the full chip set. */
function occupied(overrides: Partial<Card> = {}): Card {
  return card({
    state: "occupied",
    stay_pid: "stay-1",
    display_name: "Test Patient 001",
    edd: "2026-07-20",
    ...overrides,
  });
}

describe("BedCard states", () => {
  it.each([
    ["available", "Available"],
    ["reserved", "Reserved"],
    ["occupied", "Occupied"],
    ["awaiting_clean", "Awaiting clean"],
    ["cleaning", "Cleaning"],
    ["closed", "Closed"],
  ] as const)("renders the %s state label", (state, label) => {
    render(BedCard, {
      card: state === "occupied" ? occupied() : card({ state }),
    });
    expect(screen.getByText(label, { exact: false })).toBeTruthy();
  });

  it("shows the closure reason on a closed bed", () => {
    render(BedCard, {
      card: card({ state: "closed", closure_reason: "infection" }),
    });
    expect(screen.getByText(/\(infection\)/)).toBeTruthy();
  });

  it("shows side room + deep-clean chips on an empty bed", () => {
    render(BedCard, {
      card: card({ side_room: true, deep_clean_required: true }),
    });
    expect(screen.getByText("Side room")).toBeTruthy();
    expect(screen.getByText("Deep clean required")).toBeTruthy();
  });
});

describe("BedCard occupied chips", () => {
  it("shows the patient name and EDD", () => {
    render(BedCard, { card: occupied() });
    expect(screen.getByText("Test Patient 001")).toBeTruthy();
    expect(screen.getByText("EDD 2026-07-20")).toBeTruthy();
  });

  it("flags a missing EDD (SAFER nudge)", () => {
    render(BedCard, { card: occupied({ edd: null, edd_missing: true }) });
    expect(screen.getByText("EDD missing")).toBeTruthy();
  });

  it("flags an overdue EDD", () => {
    render(BedCard, {
      card: occupied({ edd: "2026-07-10", edd_overdue: true }),
    });
    expect(screen.getByText(/EDD 2026-07-10 overdue/)).toBeTruthy();
  });

  it("shows CCD met, pathway, and ready / DTOC", () => {
    render(BedCard, {
      card: occupied({
        ccd_met: true,
        discharge_pathway: "p1",
        discharge_ready: true,
      }),
    });
    expect(screen.getByText("CCD met")).toBeTruthy();
    expect(screen.getByText("P1")).toBeTruthy();
    expect(screen.getByText("Ready")).toBeTruthy();
  });

  it("DTOC outranks Ready", () => {
    render(BedCard, {
      card: occupied({ discharge_ready: true, dtoc: true }),
    });
    expect(screen.getByText("DTOC")).toBeTruthy();
    expect(screen.queryByText("Ready")).toBeNull();
  });

  it.each([
    ["red", "Red"],
    ["green", "Green"],
  ] as const)("shows today's %s day", (value, label) => {
    render(BedCard, { card: occupied({ red_green_today: value }) });
    expect(screen.getByText(label)).toBeTruthy();
  });

  it("shows infection chips, suspected with a question mark", () => {
    render(BedCard, {
      card: occupied({
        infection: [
          { precaution: "droplet", organism: "covid-19", status: "suspected" },
          { precaution: "contact", organism: null, status: "confirmed" },
        ],
      }),
    });
    expect(screen.getByText(/covid-19\s*\?/)).toBeTruthy();
    expect(screen.getByText("contact")).toBeTruthy();
  });

  it("shows alert chips and the senior-review tick", () => {
    render(BedCard, {
      card: occupied({ alerts: ["falls risk"], senior_review_today: true }),
    });
    expect(screen.getByText("falls risk")).toBeTruthy();
    expect(screen.getByText("Reviewed")).toBeTruthy();
  });
});

describe("BedCard masked mode", () => {
  it("redacts the name and alerts but keeps the bed state", () => {
    render(BedCard, {
      card: occupied({ alerts: ["falls risk"] }),
      masked: true,
    });
    expect(screen.getByText("•••")).toBeTruthy();
    expect(screen.queryByText("Test Patient 001")).toBeNull();
    expect(screen.queryByText("falls risk")).toBeNull();
    expect(screen.getByText("Occupied")).toBeTruthy();
  });
});

describe("BedCard actions", () => {
  it("opens the stay on patient tap", async () => {
    const onopen = vi.fn();
    render(BedCard, { card: occupied(), onopen });
    await fireEvent.click(screen.getByText("Test Patient 001"));
    expect(onopen).toHaveBeenCalledWith("stay-1");
  });

  it("offers clean-start on awaiting-clean beds", async () => {
    const oncleanstart = vi.fn();
    render(BedCard, {
      card: card({ state: "awaiting_clean" }),
      oncleanstart,
    });
    await fireEvent.click(screen.getByText("Start clean"));
    expect(oncleanstart).toHaveBeenCalledWith("bed-1");
  });

  it("routine clean-complete on a routine clean", async () => {
    const oncleancomplete = vi.fn();
    render(BedCard, { card: card({ state: "cleaning" }), oncleancomplete });
    await fireEvent.click(screen.getByText("Clean done"));
    expect(oncleancomplete).toHaveBeenCalledWith("bed-1", false);
  });

  it("deep clean-complete when a deep clean is owed", async () => {
    const oncleancomplete = vi.fn();
    render(BedCard, {
      card: card({ state: "cleaning", deep_clean_required: true }),
      oncleancomplete,
    });
    await fireEvent.click(screen.getByText("Deep clean done"));
    expect(oncleancomplete).toHaveBeenCalledWith("bed-1", true);
  });
});
