// Unit tests: WPM-T39's centralised null-not-zero ratio / percentage /
// mean formatters (`$lib/format.ts`), extracted from the per-route
// inline logic they replace.

import { describe, expect, it } from "vitest";

import { mean, percent, percentOf, percentWithWorkings, workings } from "../../src/lib/format";

describe("percent", () => {
  it("renders a present ratio as a rounded percentage", () => {
    expect(percent({ numerator: 2, denominator: 3, value: 2 / 3 })).toBe("67%");
  });

  it("renders a zero value as 0%, not as no-data", () => {
    // A real zero and an absent measurement are different claims —
    // `value: 0` means "we measured and it was zero".
    expect(percent({ numerator: 0, denominator: 5, value: 0 })).toBe("0%");
  });

  it("renders a null value (zero-denominator case) as null, never 0%", () => {
    expect(percent({ numerator: 0, denominator: 0, value: null })).toBeNull();
  });

  it("renders an absent ratio as null", () => {
    expect(percent(null)).toBeNull();
    expect(percent(undefined)).toBeNull();
  });
});

describe("workings", () => {
  it("renders the numerator/denominator pair", () => {
    expect(workings({ numerator: 2, denominator: 3, value: 2 / 3 })).toBe("2/3");
  });

  it("renders an absent ratio as null", () => {
    expect(workings(null)).toBeNull();
  });
});

describe("percentWithWorkings", () => {
  it("combines the percentage and the working", () => {
    expect(percentWithWorkings({ numerator: 2, denominator: 3, value: 2 / 3 })).toBe(
      "67% (2/3)",
    );
  });

  it("falls back to the em dash on a zero-denominator ratio", () => {
    expect(percentWithWorkings({ numerator: 0, denominator: 0, value: null })).toBe("—");
  });

  it("falls back to the em dash on an absent ratio", () => {
    expect(percentWithWorkings(null)).toBe("—");
    expect(percentWithWorkings(undefined)).toBe("—");
  });

  it("accepts a caller-supplied fallback", () => {
    expect(percentWithWorkings(null, "n/a")).toBe("n/a");
  });
});

describe("percentOf", () => {
  it("computes a percentage from a raw done/total pair", () => {
    expect(percentOf(3, 4)).toBe("75%");
  });

  it("renders a real zero as 0%, not as no-data", () => {
    expect(percentOf(0, 4)).toBe("0%");
  });

  it("falls back to the em dash on a zero total, never dividing by zero", () => {
    expect(percentOf(0, 0)).toBe("—");
  });

  it("accepts a caller-supplied fallback", () => {
    expect(percentOf(0, 0, "no steps")).toBe("no steps");
  });
});

describe("mean", () => {
  it("renders one decimal place", () => {
    expect(mean(3.44)).toBe("3.4");
  });

  it("renders a real zero as 0.0, not as absent", () => {
    expect(mean(0)).toBe("0.0");
  });

  it("renders null/undefined as null, never as the literal string \"undefined\"", () => {
    expect(mean(null)).toBeNull();
    expect(mean(undefined)).toBeNull();
  });
});
