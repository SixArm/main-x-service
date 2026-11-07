// Pins how a failed admin call becomes a message: the auth service answers
// errors as RFC 9457 problems (axa 0.6), so `detail` wins, then `title`.
import { describe, it, expect } from "vitest";
import { errorMessage } from "$lib/server/admin";

function problem(body: unknown, statusText = "Forbidden"): Response {
  return new Response(JSON.stringify(body), {
    status: 403,
    statusText,
    headers: { "content-type": "application/problem+json" },
  });
}

describe("admin errorMessage", () => {
  it("reads a problem's detail", async () => {
    const res = problem({ type: "about:blank", title: "Bad Request", status: 400, detail: "unknown key dept", code: "bad_request" });
    expect(await errorMessage(res)).toBe("unknown key dept");
  });

  it("falls back to the title when there is no detail", async () => {
    const res = problem({ type: "about:blank", title: "Forbidden", status: 403, code: "forbidden" });
    expect(await errorMessage(res)).toBe("Forbidden");
  });

  it("falls back to the status text for a body that is not JSON", async () => {
    const res = new Response("nope", { status: 502, statusText: "Bad Gateway" });
    expect(await errorMessage(res)).toBe("Bad Gateway");
  });
});
