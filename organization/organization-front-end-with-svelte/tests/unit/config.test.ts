import { describe, it, expect } from "vitest";
import { signInUrl, AUTH_FRONTEND_URL } from "$lib/config";

describe("signInUrl", () => {
  it("builds the auth front-end sign-in URL with an encoded return_to", () => {
    const url = signInUrl("http://localhost:4173", "");
    expect(url).toBe(
      `${AUTH_FRONTEND_URL}/signin?return_to=${encodeURIComponent(
        "http://localhost:4173",
      )}`,
    );
  });

  it("includes the SvelteKit base path in return_to", () => {
    const url = signInUrl("https://ops.example.com", "/orgs");
    const expected = encodeURIComponent("https://ops.example.com/orgs");
    expect(url).toContain(`return_to=${expected}`);
    // The colon and slashes of the origin must be percent-encoded so the
    // whole origin rides inside the single query param.
    expect(url).toContain("https%3A%2F%2Fops.example.com%2Forgs");
  });

  it("does not double up slashes when AUTH_FRONTEND_URL has a trailing slash", () => {
    // signInUrl trims a trailing slash off the configured base before
    // appending `/signin`.
    const url = signInUrl("http://localhost:4173", "");
    expect(url).not.toContain("//signin");
    expect(url).toContain("/signin?return_to=");
  });
});
