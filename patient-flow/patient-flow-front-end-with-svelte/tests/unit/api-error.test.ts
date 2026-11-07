// Pins ApiError's message: the service answers errors as RFC 9457 problems
// (axa 0.6), so `detail` wins, then `title`, then the status.
import { describe, expect, it } from "vitest";
import { ApiError } from "../../src/lib/api/client";

describe("ApiError", () => {
  it("takes the problem's detail", () => {
    const err = new ApiError(409, {
      type: "about:blank",
      title: "Conflict",
      status: 409,
      detail: "bed already occupied",
      code: "conflict",
    });
    expect(err.message).toBe("bed already occupied");
    expect(err.status).toBe(409);
  });

  it("falls back to the title when there is no detail", () => {
    const err = new ApiError(500, {
      type: "about:blank",
      title: "Internal Server Error",
      status: 500,
      code: "internal_server_error",
    });
    expect(err.message).toBe("Internal Server Error");
  });

  it("falls back to the status for a body that is not a problem", () => {
    expect(new ApiError(502, null).message).toBe("API error 502");
  });
});
