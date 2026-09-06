// Unit tests for describeApiError (T-29): the shared 401/403 reaction
// every route's catch block now delegates to.
import { beforeEach, describe, expect, it, vi } from "vitest";

// i18n reads `browser` from $app/environment to decide whether to touch
// localStorage; off the browser it just seeds the default (English) locale.
vi.mock("$app/environment", () => ({ browser: false }));

// `vi.mock` factories are hoisted above imports, so the mock fn itself
// must be created inside `vi.hoisted` to avoid a temporal-dead-zone
// reference error.
const { gotoMock } = vi.hoisted(() => ({ gotoMock: vi.fn() }));
vi.mock("$app/navigation", () => ({ goto: gotoMock }));

import { ApiError } from "../../src/lib/api/client";
import { describeApiError } from "../../src/lib/api/errorHandling";

describe("describeApiError", () => {
    beforeEach(() => {
        gotoMock.mockClear();
    });

    // Pins: a 401 redirects to /signin and returns the translated
    // session-expired message, not the raw server error body.
    it("redirects to /signin on 401 and returns a translated message", () => {
        const err = new ApiError(401, { code: "UNAUTHORIZED", message: "no token" });
        const message = describeApiError(err);
        expect(gotoMock).toHaveBeenCalledTimes(1);
        expect(gotoMock).toHaveBeenCalledWith("/signin");
        expect(message).toBe("Your session has expired. Redirecting to sign in…");
    });

    // Pins: a 403 shows a translated access-denied message and does NOT
    // redirect — the session is valid, just not authorized.
    it("returns a translated access-denied message on 403 without redirecting", () => {
        const err = new ApiError(403, { code: "FORBIDDEN", message: "policy denied" });
        const message = describeApiError(err);
        expect(gotoMock).not.toHaveBeenCalled();
        expect(message).toBe("You don't have permission to do that.");
    });

    // Pins: every other ApiError status falls back to its own message,
    // matching every route's previous inline behaviour exactly.
    it("falls back to the error's own message for a non-auth ApiError", () => {
        const err = new ApiError(404, { code: "NOT_FOUND", message: "missing" });
        const message = describeApiError(err);
        expect(gotoMock).not.toHaveBeenCalled();
        expect(message).toBe("missing");
    });

    // Pins: a plain Error (not an ApiError — e.g. a network failure)
    // still falls back to its own message.
    it("falls back to a plain Error's message", () => {
        const message = describeApiError(new Error("network down"));
        expect(gotoMock).not.toHaveBeenCalled();
        expect(message).toBe("network down");
    });

    // Pins: a non-Error thrown value stringifies, matching the previous
    // `String(err)` fallback every route used.
    it("stringifies a non-Error thrown value", () => {
        const message = describeApiError("weird throw");
        expect(gotoMock).not.toHaveBeenCalled();
        expect(message).toBe("weird throw");
    });
});
