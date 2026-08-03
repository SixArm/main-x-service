// Unit tests for the client-side cross-service link rules. These mirror
// the accept/reject matrix pinned on the Rust side by
// `person-service-with-loco/src/api/rest/links.rs::validate_edge`, so a
// drift in either direction shows up as a failing test rather than as a
// 422 the operator has to interpret.
import { describe, expect, it } from "vitest";
import {
    PERSON_LINK_KINDS,
    expectedTargetType,
    isPersonLinkKind,
    parseEntityRef,
    refPlaceholder,
    validateToRef,
} from "../../src/lib/links";

const WORKER = "worker:0c4f1e2a-0000-4000-8000-000000000000";
const PERSON = "person:0c4f1e2a-0000-4000-8000-000000000001";
const ORG = "organization:0c4f1e2a-0000-4000-8000-000000000002";

describe("person link kinds", () => {
    // Pins the exact set person may originate: `employed_by` is
    // worker-originated and `subject_of` is case-originated, so neither
    // may appear in this UI.
    it("offers exactly the three kinds person originates", () => {
        expect([...PERSON_LINK_KINDS]).toEqual([
            "same_identity",
            "works_at",
            "member_of",
        ]);
        expect(isPersonLinkKind("same_identity")).toBe(true);
        expect(isPersonLinkKind("employed_by")).toBe(false);
        expect(isPersonLinkKind("subject_of")).toBe(false);
    });

    // Pins the kind → target-type table the server enforces.
    it("maps each kind to its required target type", () => {
        expect(expectedTargetType("same_identity")).toBe("worker");
        expect(expectedTargetType("works_at")).toBe("organization");
        expect(expectedTargetType("member_of")).toBe("organization");
    });

    // The placeholder teaches the URN shape without a round-trip.
    it("derives a placeholder from the expected target type", () => {
        expect(refPlaceholder("same_identity")).toBe("worker:<uuid>");
        expect(refPlaceholder("works_at")).toBe("organization:<uuid>");
    });
});

describe("parseEntityRef", () => {
    // Mirrors the Rust `EntityRef::from_str`: one `:`, non-empty type,
    // canonical UUID id.
    it("splits a well-formed URN into type and id", () => {
        expect(parseEntityRef(WORKER)).toEqual({
            entityType: "worker",
            id: "0c4f1e2a-0000-4000-8000-000000000000",
        });
    });

    it("accepts an upper-case UUID", () => {
        expect(
            parseEntityRef("worker:0C4F1E2A-0000-4000-8000-000000000000"),
        ).not.toBeNull();
    });

    it("rejects malformed refs", () => {
        for (const bad of [
            "not-a-ref",
            "worker:",
            "worker:not-a-uuid",
            ":0c4f1e2a-0000-4000-8000-000000000000",
            "worker:0c4f1e2a-0000-4000-8000-000000000000:extra",
            "",
        ]) {
            expect(parseEntityRef(bad), bad).toBeNull();
        }
    });
});

describe("validateToRef", () => {
    // The happy paths: the `same_identity` backbone and the two
    // affiliations.
    it("accepts the target type each kind requires", () => {
        expect(validateToRef("same_identity", WORKER)).toBeNull();
        expect(validateToRef("works_at", ORG)).toBeNull();
        expect(validateToRef("member_of", ORG)).toBeNull();
    });

    it("trims surrounding whitespace before judging", () => {
        expect(validateToRef("same_identity", `  ${WORKER}  `)).toBeNull();
    });

    it("reports an empty target as required", () => {
        expect(validateToRef("same_identity", "")).toBe("required");
        expect(validateToRef("same_identity", "   ")).toBe("required");
    });

    it("reports a non-URN target as malformed", () => {
        expect(validateToRef("same_identity", "0c4f1e2a")).toBe("malformed");
        expect(validateToRef("works_at", "organization:nope")).toBe("malformed");
    });

    // The class of mistake this whole helper exists to catch early:
    // a valid ref pointing at the wrong kind of record.
    it("reports a valid ref of the wrong entity type", () => {
        expect(validateToRef("same_identity", PERSON)).toBe("wrong-type");
        expect(validateToRef("same_identity", ORG)).toBe("wrong-type");
        expect(validateToRef("works_at", WORKER)).toBe("wrong-type");
        expect(validateToRef("member_of", PERSON)).toBe("wrong-type");
    });

    // An unrecognised entity type is surfaced as "wrong type" rather than
    // "malformed": from the form's point of view the actionable statement
    // is which type this kind wants.
    it("treats an unknown entity type as the wrong type", () => {
        expect(
            validateToRef("same_identity", "widget:0c4f1e2a-0000-4000-8000-000000000000"),
        ).toBe("wrong-type");
    });
});
