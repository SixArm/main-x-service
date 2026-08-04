// Unit tests for the pure duplicate-review rules in `$lib/review` — the
// decidable-status guard, the score-breakdown mapper, the boolean-flag
// mapper, and the merge deep link. All pure, so no Svelte component and no
// network.
import { describe, expect, it } from "vitest";
import {
    MATCH_COMPONENTS,
    REVIEW_LIMITS,
    REVIEW_STATUSES,
    breakdownFlags,
    breakdownRows,
    canDecide,
    isReviewStatus,
    mergeHref,
} from "../../src/lib/review";

// The five weighted components `MatchBreakdown` carries, plus its two
// boolean flags — the shape `thing-service-with-loco`'s ad-hoc match /
// check-duplicates endpoints already return (even though the review-queue
// endpoint does not wire `score_breakdown` through yet; see the module doc
// in `src/lib/review.ts`).
const fullBreakdown = {
    name_score: 0.94,
    identifier_score: 0.0,
    description_score: 0.6,
    url_score: 1.0,
    same_as_score: 0.0,
    phonetic_match: true,
    deterministic_match: false,
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
    // Pins the service's `MatchWeights::default()`
    // (`thing-service-with-loco/src/matching/scoring.rs`). If they no
    // longer sum to 1.00 the table is lying to the operator.
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

    it("names the five wire keys the service serializes", () => {
        expect(MATCH_COMPONENTS.map((c) => c.key)).toEqual([
            "name_score",
            "identifier_score",
            "description_score",
            "url_score",
            "same_as_score",
        ]);
    });
});

describe("breakdownRows", () => {
    it("maps a full breakdown to one row per component, in weight order", () => {
        const rows = breakdownRows(fullBreakdown);
        expect(rows).toHaveLength(5);
        expect(rows[0]?.key).toBe("name_score");
        expect(rows[0]?.score).toBe(0.94);
        expect(rows[0]?.weight).toBe(0.4);
        expect(rows.at(-1)?.key).toBe("same_as_score");
    });

    // Pins: a `null` breakdown (the service sends `Option<Value>`, and in
    // fact never populates it on the review-queue wire type today — see
    // the module doc) yields no rows, so the page can render an explicit
    // note rather than an empty table or a crash.
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
        const rows = breakdownRows({ name_score: 0.8, url_score: 1.0 });
        expect(rows.map((r) => r.key)).toEqual(["name_score", "url_score"]);
    });

    // Pins: an unknown key is ignored rather than rendered as a mystery
    // row, and a non-finite value is not rendered as "NaN".
    it("ignores unknown keys and non-numeric values", () => {
        const rows = breakdownRows({
            name_score: 0.8,
            future_score: 0.99,
            url_score: "1.0",
            description_score: Number.NaN,
        });
        expect(rows.map((r) => r.key)).toEqual(["name_score"]);
    });
});

describe("breakdownFlags", () => {
    it("returns only the flags that are true", () => {
        const flags = breakdownFlags(fullBreakdown);
        expect(flags.map((f) => f.key)).toEqual(["phonetic_match"]);
    });

    it("returns no flags when both are false or absent", () => {
        expect(breakdownFlags({ name_score: 0.5 })).toEqual([]);
        expect(
            breakdownFlags({ phonetic_match: false, deterministic_match: false }),
        ).toEqual([]);
    });

    it("returns no flags for a null / non-object payload", () => {
        expect(breakdownFlags(null)).toEqual([]);
        expect(breakdownFlags(undefined)).toEqual([]);
        expect(breakdownFlags("true")).toEqual([]);
    });

    // Pins: a truthy-but-not-`true` value (e.g. the string "true") must not
    // read as the flag being set — only a literal boolean `true` counts.
    it("does not treat a truthy string as the flag being set", () => {
        expect(breakdownFlags({ phonetic_match: "true" })).toEqual([]);
    });
});

describe("mergeHref", () => {
    // Pins: the parameter names the merge page reads to pre-fill its two
    // id inputs. Renaming either silently breaks the deep link.
    it("builds a /things/merge link carrying both ids", () => {
        const href = mergeHref("aaa-111", "bbb-222");
        const url = new URL(href, "http://test");
        expect(url.pathname).toBe("/things/merge");
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
