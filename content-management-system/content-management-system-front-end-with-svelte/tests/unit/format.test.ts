// Formatters. The honesty rules live here, so they are tested here.

import { describe, expect, it } from "vitest";
import {
  actor,
  bytes,
  duration,
  percent,
  staleness,
  when,
  workings,
} from "$lib/format";

describe("formatters", () => {
  it("renders no data as no data, never as zero", () => {
    // The distinction the insights spec cares about most: "we measured
    // and it was zero" is a different claim from "there was nothing to
    // measure".
    expect(percent({ numerator: 0, denominator: 0, value: null })).toBeNull();
    expect(percent(null)).toBeNull();
    expect(percent(undefined)).toBeNull();
    expect(percent({ numerator: 0, denominator: 12, value: 0 })).toBe("0%");
    expect(percent({ numerator: 13, denominator: 15, value: 13 / 15 })).toBe(
      "87%",
    );
  });

  it("shows a ratio's working", () => {
    expect(workings({ numerator: 13, denominator: 15, value: 0.86 })).toBe(
      "13 / 15",
    );
    expect(workings(null)).toBeNull();
  });

  it("renders durations at a sensible scale", () => {
    expect(duration(45)).toBe("45s");
    expect(duration(600)).toBe("10m");
    expect(duration(7200)).toBe("2h");
    expect(duration(432_000)).toBe("5d");
    expect(duration(null)).toBeNull();
    expect(duration(undefined)).toBeNull();
  });

  it("renders byte sizes", () => {
    expect(bytes(512)).toBe("512 B");
    expect(bytes(26_788_000)).toBe("27 MB");
    expect(bytes(1_500)).toBe("1.5 kB");
    expect(bytes(0)).toBe("0 B");
  });

  it("keeps unknown staleness distinct from up-to-date", () => {
    // Collapsing these would tell an editor a translation is fine when
    // in fact nobody knows.
    expect(staleness({ stale: false, revisions_behind: 0 }).tone).toBe("ok");
    expect(
      staleness({
        stale: false,
        revisions_behind: 0,
        unknown: "this variant does not record a source revision",
      }).tone,
    ).toBe("unknown");
    expect(staleness(null).tone).toBe("unknown");
    expect(staleness(undefined).tone).toBe("unknown");
  });

  it("counts how far behind, with the plural right", () => {
    expect(staleness({ stale: true, revisions_behind: 1 }).text).toBe(
      "1 source revision behind",
    );
    expect(staleness({ stale: true, revisions_behind: 3 }).text).toBe(
      "3 source revisions behind",
    );
  });

  it("shortens an actor reference without pretending to know a name", () => {
    expect(actor("worker:11111111-1111-4111-8111-111111111111")).toBe(
      "worker:11111111…",
    );
    expect(actor(null)).toBe("unattributed");
    expect(actor("plain")).toBe("plain");
  });

  it("formats a timestamp, or nothing at all", () => {
    expect(when(null)).toBe("");
    expect(when(undefined)).toBe("");
    expect(when("2026-07-31T09:00:00Z").length).toBeGreaterThan(0);
  });
});
