// Unit tests for the links panel's pure guards. These mirror the
// service-side accept/reject matrix in `case-service-with-loco`
// (`src/controllers/links.rs::validate_edge`), so the client refuses
// exactly what the service would refuse — a client that were laxer would
// surface a confusing 422, and a client that were stricter would block a
// legitimate assertion.
import { describe, it, expect } from "vitest";
import {
  isPersonRef,
  validateLink,
  PERSON_REF_PATTERN,
} from "$lib/components/link-validation";

const PERSON = "person:0c4f1e2a-0000-4000-8000-000000000000";

describe("isPersonRef", () => {
  it("accepts a canonical person EntityRef URN", () => {
    expect(isPersonRef(PERSON)).toBe(true);
  });

  it("accepts an uppercase UUID and tolerates surrounding whitespace", () => {
    expect(isPersonRef("person:0C4F1E2A-0000-4000-8000-000000000000")).toBe(
      true,
    );
    expect(isPersonRef(`  ${PERSON}  `)).toBe(true);
  });

  it("rejects a bare UUID with no entity type", () => {
    expect(isPersonRef("0c4f1e2a-0000-4000-8000-000000000000")).toBe(false);
  });

  it("rejects other entity types (a case may only assert subject_of → person)", () => {
    // Mirrors the service's rejection of a non-person `to_ref`.
    expect(isPersonRef("case:0c4f1e2a-0000-4000-8000-000000000001")).toBe(
      false,
    );
    expect(isPersonRef("worker:0c4f1e2a-0000-4000-8000-000000000001")).toBe(
      false,
    );
  });

  it("rejects malformed refs the service also rejects", () => {
    for (const bad of ["", "not-a-ref", "person:", "person:123", "widget:123"]) {
      expect(isPersonRef(bad), bad).toBe(false);
    }
  });

  it("anchors the pattern so a ref cannot be smuggled in a longer string", () => {
    expect(PERSON_REF_PATTERN.test(`x${PERSON}`)).toBe(false);
    expect(PERSON_REF_PATTERN.test(`${PERSON}x`)).toBe(false);
  });
});

describe("validateLink", () => {
  it("passes a valid ref with no confidence", () => {
    expect(validateLink(PERSON, null)).toBeNull();
  });

  it("passes the confidence bounds inclusively", () => {
    expect(validateLink(PERSON, 0)).toBeNull();
    expect(validateLink(PERSON, 1)).toBeNull();
    expect(validateLink(PERSON, 0.87)).toBeNull();
  });

  it("returns the ref key for a bad person reference", () => {
    expect(validateLink("nope", null)).toBe("links.invalidPersonRef");
  });

  it("returns the range key for an out-of-range confidence", () => {
    expect(validateLink(PERSON, -0.1)).toBe("links.confidenceRange");
    expect(validateLink(PERSON, 1.5)).toBe("links.confidenceRange");
    expect(validateLink(PERSON, Number.NaN)).toBe("links.confidenceRange");
  });

  it("reports the bad reference first when both are wrong", () => {
    expect(validateLink("nope", 9)).toBe("links.invalidPersonRef");
  });
});
