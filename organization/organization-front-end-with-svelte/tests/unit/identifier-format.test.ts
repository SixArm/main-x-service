// Unit tests for the pure identifierFormatHint() helper (ORGFE-T4): a
// client-side length/format hint mirroring the shape checks in
// organization-service-with-loco's SEC-M5 validation
// (`identifier_problem`), without re-implementing any check digit. The
// server stays authoritative — this only catches an obviously
// malformed value before the round trip.
import { describe, expect, it } from "vitest";
import { identifierFormatHint } from "../../src/lib/identifier-format";

describe("identifierFormatHint", () => {
  describe("Lei", () => {
    it("accepts a 20-character alphanumeric value", () => {
      expect(identifierFormatHint("Lei", "5493001KJTIIGC8Y1R12")).toBeNull();
    });

    it("flags a value with the wrong length", () => {
      expect(identifierFormatHint("Lei", "TOOSHORT")).toBe(
        "Expected 20 alphanumeric characters",
      );
    });

    it("flags a 20-character value with a non-alphanumeric character", () => {
      expect(identifierFormatHint("Lei", "5493001KJTIIGC8Y1R-2")).toBe(
        "Expected 20 alphanumeric characters",
      );
    });

    it("is case-insensitive", () => {
      expect(identifierFormatHint("Lei", "5493001kjtiigc8y1r12")).toBeNull();
    });
  });

  describe("Duns", () => {
    it("accepts 9 digits", () => {
      expect(identifierFormatHint("Duns", "123456789")).toBeNull();
    });

    it("accepts 9 digits with hyphens", () => {
      expect(identifierFormatHint("Duns", "12-345-6789")).toBeNull();
    });

    it("flags the wrong digit count", () => {
      expect(identifierFormatHint("Duns", "12345")).toBe("Expected 9 digits");
    });

    it("flags a non-digit character", () => {
      expect(identifierFormatHint("Duns", "12345678X")).toBe("Expected 9 digits");
    });
  });

  describe("Gln", () => {
    it("accepts 13 digits", () => {
      expect(identifierFormatHint("Gln", "1234567890128")).toBeNull();
    });

    it("flags the wrong digit count", () => {
      expect(identifierFormatHint("Gln", "123456789")).toBe("Expected 13 digits");
    });
  });

  describe("Vat", () => {
    it("accepts a 2-letter prefix plus alphanumerics", () => {
      expect(identifierFormatHint("Vat", "GB123456789")).toBeNull();
    });

    it("flags a value with no letter prefix", () => {
      expect(identifierFormatHint("Vat", "123456789")).toBe(
        "Expected a 2-letter country prefix followed by 2–13 alphanumerics",
      );
    });

    it("flags a value that is too short", () => {
      expect(identifierFormatHint("Vat", "GB1")).toBe(
        "Expected a 2-letter country prefix followed by 2–13 alphanumerics",
      );
    });

    it("flags a value that is too long", () => {
      expect(identifierFormatHint("Vat", "GB1234567890123456")).toBe(
        "Expected a 2-letter country prefix followed by 2–13 alphanumerics",
      );
    });
  });

  it("returns null for blank values (dropped on submit, not a format error)", () => {
    expect(identifierFormatHint("Lei", "   ")).toBeNull();
  });

  it("returns null for unconstrained schemes (TaxId, Naics, IsicV4, Sic)", () => {
    expect(identifierFormatHint("TaxId", "anything")).toBeNull();
    expect(identifierFormatHint("Naics", "x")).toBeNull();
    expect(identifierFormatHint("IsicV4", "x")).toBeNull();
    expect(identifierFormatHint("Sic", "x")).toBeNull();
  });

  it("returns null for the Custom variant", () => {
    expect(identifierFormatHint({ Custom: "internal-ref" }, "anything")).toBeNull();
  });

  it("returns null for deterministic-but-unvalidated schemes (Iso6523, Wikidata, Ror, Isni)", () => {
    expect(identifierFormatHint("Iso6523", "anything")).toBeNull();
    expect(identifierFormatHint("Wikidata", "anything")).toBeNull();
    expect(identifierFormatHint("Ror", "anything")).toBeNull();
    expect(identifierFormatHint("Isni", "anything")).toBeNull();
  });
});
