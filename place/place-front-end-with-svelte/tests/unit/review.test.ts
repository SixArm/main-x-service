// Unit tests for the pure duplicate-review rules in `$lib/review` — the
// decidable-status guard, the score-breakdown mapper, and the merge deep
// link. All pure, so no Svelte component and no network.
import { describe, expect, it } from "vitest";
import {
    MATCH_COMPONENTS,
    REVIEW_LIMITS,
    REVIEW_STATUSES,
    breakdownRows,
    canDecide,
    isReviewStatus,
    mergeHref,
} from "../../src/lib/review";

// The five components place-matcher's `MatchBreakdown` carries, with
// plausible values a real scan would produce.
const fullBreakdown = {
    name_score: 0.94,
    geo_score: 0.8,
    address_score: 0.42,
    place_type_score: 1.0,
    identifier_score: 0.0,
};

describe("review status vocabulary", () => {
    // Pins: the four wire tokens, in the order the columns and the filter
    // present them. A typo here is a 422 INVALID_STATUS at runtime.
    it("lists exactly the four stored dispositions", () => {
        expect([...REVIEW_STATUSES]).toEqual([
            "pending",
            "confirmed",
            "rejected",
            "automerged",
        ]);
    });

    it("recognises only those four tokens", () => {
        expect(isReviewStatus("pending")).toBe(true);
        expect(isReviewStatus("automerged")).toBe(true);
        // The Rust variant is `AutoMerged`, but the wire token is one word.
        expect(isReviewStatus("auto_merged")).toBe(false);
        expect(isReviewStatus("all")).toBe(false);
        expect(isReviewStatus("")).toBe(false);
    });

    // Pins: nothing above the service's own 500 cap is offered, since a
    // larger request would be silently clamped rather than honoured.
    it("offers no page size above the service cap", () => {
        expect(Math.max(...REVIEW_LIMITS)).toBe(500);
    });
});

describe("canDecide", () => {
    // Pins: only `pending` is decidable — the service's update is guarded
    // by `WHERE status = 'pending'` and answers 422 otherwise.
    it("allows a pending item", () => {
        expect(canDecide({ status: "pending" })).toBe(true);
    });

    it("refuses an already-decided item", () => {
        expect(canDecide({ status: "confirmed" })).toBe(false);
        expect(canDecide({ status: "rejected" })).toBe(false);
        expect(canDecide({ status: "automerged" })).toBe(false);
    });
});

describe("MATCH_COMPONENTS", () => {
    // Pins the place matcher's published default weights
    // (agents/matching.md). If they no longer sum to 1.00 the table is
    // lying to the operator.
    it("weights sum to 1.00", () => {
        const total = MATCH_COMPONENTS.reduce((sum, c) => sum + c.weight, 0);
        expect(total).toBeCloseTo(1.0, 10);
    });

    // Pins: descending weight, so the rows scanned first are the ones that
    // actually moved the score.
    it("is ordered by descending weight", () => {
        const weights = MATCH_COMPONENTS.map((c) => c.weight);
        expect([...weights].sort((a, b) => b - a)).toEqual(weights);
    });

    it("names the five wire keys place-matcher's MatchBreakdown carries", () => {
        expect(MATCH_COMPONENTS.map((c) => c.key)).toEqual([
            "name_score",
            "geo_score",
            "address_score",
            "place_type_score",
            "identifier_score",
        ]);
    });
});

describe("breakdownRows", () => {
    it("maps a full breakdown to one row per component, in weight order", () => {
        const rows = breakdownRows(fullBreakdown);
        expect(rows).toHaveLength(5);
        expect(rows[0]?.key).toBe("name_score");
        expect(rows[0]?.score).toBe(0.94);
        expect(rows[0]?.weight).toBe(0.35);
        expect(rows.at(-1)?.key).toBe("identifier_score");
    });

    // Pins: place-service does not currently serialize `score_breakdown`
    // on the wire at all (see the module's KNOWN GAP doc comment), so the
    // page calls this with `undefined` today — it must resolve to no
    // rows, not a crash, so the panel can render an explicit note.
    it("returns no rows for a null / absent breakdown", () => {
        expect(breakdownRows(null)).toEqual([]);
        expect(breakdownRows(undefined)).toEqual([]);
    });

    // Pins: a non-object payload is data we do not understand, not an
    // exception. This runs on whatever the service sent.
    it("returns no rows for a non-object payload", () => {
        expect(breakdownRows("0.9")).toEqual([]);
        expect(breakdownRows(0.9)).toEqual([]);
        expect(breakdownRows([0.9])).toEqual([]);
    });

    // Pins: a missing component is omitted rather than shown as 0.00 —
    // "not compared" must not read as "compared and did not match".
    it("omits components the payload does not carry", () => {
        const rows = breakdownRows({ name_score: 0.8, geo_score: 1.0 });
        expect(rows.map((r) => r.key)).toEqual(["name_score", "geo_score"]);
    });

    // Pins: an unknown key is ignored rather than rendered as a mystery
    // row, and a non-finite value is not rendered as "NaN".
    it("ignores unknown keys and non-numeric values", () => {
        const rows = breakdownRows({
            name_score: 0.8,
            future_score: 0.99,
            geo_score: "1.0",
            address_score: Number.NaN,
        });
        expect(rows.map((r) => r.key)).toEqual(["name_score"]);
    });
});

describe("mergeHref", () => {
    // Pins: the parameter names the merge page reads to pre-fill its two
    // id inputs. Renaming either silently breaks the deep link.
    it("builds a /places/merge link carrying both ids", () => {
        const href = mergeHref("aaa-111", "bbb-222");
        const url = new URL(href, "http://test");
        expect(url.pathname).toBe("/places/merge");
        expect(url.searchParams.get("main")).toBe("aaa-111");
        expect(url.searchParams.get("duplicate")).toBe("bbb-222");
    });

    // Pins: the pair is unordered, so the caller chooses the survivor and
    // swapping the arguments swaps the roles.
    it("swaps the roles when the arguments are swapped", () => {
        const url = new URL(mergeHref("bbb-222", "aaa-111"), "http://test");
        expect(url.searchParams.get("main")).toBe("bbb-222");
        expect(url.searchParams.get("duplicate")).toBe("aaa-111");
    });
});
