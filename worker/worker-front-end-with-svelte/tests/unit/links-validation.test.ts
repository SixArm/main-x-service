// Unit tests for the client-side cross-service link pre-checks. These
// mirror the worker service's `validate_edge` (src/api/rest/links.rs), so
// the accept/reject matrix here is kept deliberately parallel to the Rust
// test module — if the service's permitted kinds change, both fail.
import { describe, expect, it } from "vitest";
import {
    EDGE_KIND_TARGET,
    WORKER_EDGE_KINDS,
    checkConfidence,
    checkToRef,
    targetEntityType,
    targetRefExample,
} from "../../src/lib/api/links";

const PERSON = "person:0c4f1e2a-0000-4000-8000-000000000000";
const WORKER = "worker:0c4f1e2a-0000-4000-8000-000000000001";
const ORG = "organization:0c4f1e2a-0000-4000-8000-000000000002";

describe("worker edge kinds", () => {
    // Pins: exactly the two kinds worker may originate, in picker order.
    it("offers exactly same_identity and employed_by", () => {
        expect([...WORKER_EDGE_KINDS]).toEqual([
            "same_identity",
            "employed_by",
        ]);
    });

    // Pins: the required target entity type per kind (§9 registry).
    it("maps each kind to its required target type", () => {
        expect(EDGE_KIND_TARGET.same_identity).toBe("person");
        expect(EDGE_KIND_TARGET.employed_by).toBe("organization");
        expect(targetEntityType("employed_by")).toBe("organization");
    });

    // Pins: the placeholder hint names the target type, so an operator
    // sees the requirement before submitting.
    it("builds a target-shaped example ref", () => {
        expect(targetRefExample("same_identity")).toBe("person:<uuid>");
        expect(targetRefExample("employed_by")).toBe("organization:<uuid>");
    });
});

describe("checkToRef", () => {
    // The same_identity backbone: worker → person.
    it("accepts a person ref for same_identity", () => {
        expect(checkToRef("same_identity", PERSON)).toBeNull();
    });

    // The employed_by affiliation: worker → organization.
    it("accepts an organization ref for employed_by", () => {
        expect(checkToRef("employed_by", ORG)).toBeNull();
    });

    // Surrounding whitespace is tolerated (the panel trims before POSTing).
    it("tolerates surrounding whitespace", () => {
        expect(checkToRef("same_identity", `  ${PERSON}  `)).toBeNull();
    });

    // The entity-type token is case-insensitive; the id is not reformatted.
    it("accepts an upper-case entity type token", () => {
        expect(checkToRef("same_identity", PERSON.toUpperCase())).toBeNull();
    });

    // An empty field is "required", distinct from malformed, so the panel
    // can show a plain prompt rather than a format complaint.
    it("reports an empty ref as required", () => {
        expect(checkToRef("same_identity", "")).toBe("required");
        expect(checkToRef("same_identity", "   ")).toBe("required");
    });

    // Mirrors the Rust `rejects_malformed_to_ref` case list.
    it("rejects malformed refs", () => {
        for (const bad of [
            "not-a-ref",
            "person:",
            ":0c4f1e2a-0000-4000-8000-000000000000",
            "person:not-a-uuid",
            "person:0c4f1e2a-0000-4000-8000",
        ]) {
            expect(checkToRef("same_identity", bad)).toBe("malformed");
        }
    });

    // Mirrors `rejects_same_identity_to_non_person` /
    // `rejects_employed_by_to_non_org`: well-formed, wrong endpoint type.
    it("rejects a well-formed ref of the wrong target type", () => {
        expect(checkToRef("same_identity", WORKER)).toBe("wrong_target");
        expect(checkToRef("same_identity", ORG)).toBe("wrong_target");
        expect(checkToRef("employed_by", PERSON)).toBe("wrong_target");
        expect(checkToRef("employed_by", WORKER)).toBe("wrong_target");
    });

    // An unknown entity type is reported as the wrong target rather than
    // malformed — the message names the type the kind actually needs.
    it("rejects an unknown entity type as wrong_target", () => {
        const ref = "widget:0c4f1e2a-0000-4000-8000-000000000003";
        expect(checkToRef("same_identity", ref)).toBe("wrong_target");
    });
});

describe("checkConfidence", () => {
    // A blank number input binds to null, meaning "not supplied".
    it("accepts null (field left blank)", () => {
        expect(checkConfidence(null)).toBeNull();
    });

    // The service stores confidence in [0.0, 1.0]; the bounds are valid.
    it("accepts values within [0, 1] inclusive", () => {
        for (const value of [0, 0.5, 1]) {
            expect(checkConfidence(value)).toBeNull();
        }
    });

    // Out-of-range and non-finite values are refused before the POST.
    it("rejects values outside [0, 1] and non-finite input", () => {
        for (const value of [-0.1, 1.1, 42, Number.NaN, Number.POSITIVE_INFINITY]) {
            expect(checkConfidence(value)).toBe("invalid");
        }
    });
});
