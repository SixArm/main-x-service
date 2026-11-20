// Pins the SSO-only mode (spec/nhs-wales-supplier-assurance NWS-A3):
// with PUBLIC_MAGIC_LINK_SIGNIN_ENABLED=false the sign-up page and both
// magic-link actions answer 404, matching the auth service, which mounts
// no issuance route in that mode. Unset keeps magic links on.
import { describe, expect, it, vi, beforeEach } from "vitest";
import { isHttpError } from "@sveltejs/kit";
import { magicLinkEnabled } from "../../src/lib/magic-link";

const mockEnv: { PUBLIC_MAGIC_LINK_SIGNIN_ENABLED?: string } = {};

vi.mock("$app/env/public", () => ({
  get PUBLIC_MAGIC_LINK_SIGNIN_ENABLED() {
    return mockEnv.PUBLIC_MAGIC_LINK_SIGNIN_ENABLED;
  },
}));

vi.mock("#lib/server/auth.js", () => ({
  requestMagicLink: vi.fn(async () => "sent"),
  signup: vi.fn(async () => "sent"),
}));

function formRequest(): Request {
  const body = new FormData();
  body.set("email", "a@example.com");
  return new Request("https://auth-front-end.example.test/", {
    method: "POST",
    body,
  });
}

async function status404(run: () => unknown): Promise<void> {
  try {
    await run();
    throw new Error("expected a 404");
  } catch (err) {
    expect(isHttpError(err)).toBe(true);
    if (isHttpError(err)) expect(err.status).toBe(404);
  }
}

describe("magicLinkEnabled", () => {
  it("is on unless the value is false", () => {
    for (const on of [undefined, "", "true", "1", "yes"])
      expect(magicLinkEnabled(on)).toBe(true);
    for (const off of ["false", "FALSE", " false "])
      expect(magicLinkEnabled(off)).toBe(false);
  });
});

describe("SSO-only mode", () => {
  beforeEach(() => {
    mockEnv.PUBLIC_MAGIC_LINK_SIGNIN_ENABLED = "false";
  });

  it("/signup load answers 404", async () => {
    const { load } = await import("../../src/routes/signup/+page.server");
    await status404(() => load({} as Parameters<typeof load>[0]));
  });

  it("the signup and signin actions answer 404", async () => {
    const signup = await import("../../src/routes/signup/+page.server");
    const signin = await import("../../src/routes/signin/+page.server");
    for (const action of [signup.actions.default, signin.actions.default]) {
      await status404(() =>
        action({ request: formRequest(), fetch } as unknown as Parameters<
          typeof action
        >[0]),
      );
    }
  });
});

describe("default mode", () => {
  beforeEach(() => {
    delete mockEnv.PUBLIC_MAGIC_LINK_SIGNIN_ENABLED;
  });

  it("the signin action still sends a link", async () => {
    const { actions } = await import("../../src/routes/signin/+page.server");
    const result = await actions.default({
      request: formRequest(),
      fetch,
    } as unknown as Parameters<typeof actions.default>[0]);
    expect(result).toEqual({ sent: true, error: null });
  });
});
