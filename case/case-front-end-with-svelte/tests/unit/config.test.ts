// Unit tests for signInUrl. Pins the cross-origin SSO handoff URL shape:
// `${AUTH_FRONTEND_URL}/signin?return_to=<encoded origin+base>`, including
// base-path inclusion and trailing-slash de-duplication.
import { describe, it, expect } from "vitest";
import { signInUrl, AUTH_FRONTEND_URL } from "$lib/config";

describe("signInUrl", () => {
  // Pins: origin is encoded into the single return_to query param.
  it("builds the auth front-end sign-in URL with an encoded return_to", () => {
    const url = signInUrl("http://localhost:4173", "");
    expect(url).toBe(
      `${AUTH_FRONTEND_URL}/signin?return_to=${encodeURIComponent(
        "http://localhost:4173",
      )}`,
    );
  });

  // Pins: the SvelteKit base path rides inside the encoded return_to.
  it("includes the SvelteKit base path in return_to", () => {
    const url = signInUrl("https://ops.example.com", "/cases");
    const expected = encodeURIComponent("https://ops.example.com/cases");
    expect(url).toContain(`return_to=${expected}`);
    // The colon and slashes of the origin must be percent-encoded so the
    // whole origin rides inside the single query param.
    expect(url).toContain("https%3A%2F%2Fops.example.com%2Fcases");
  });

  // Pins: a trailing slash on the configured base never yields `//signin`.
  it("does not double up slashes when AUTH_FRONTEND_URL has a trailing slash", () => {
    // signInUrl trims a trailing slash off the configured base before
    // appending `/signin`.
    const url = signInUrl("http://localhost:4173", "");
    expect(url).not.toContain("//signin");
    expect(url).toContain("/signin?return_to=");
  });
});
