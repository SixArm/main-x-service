// Real magic-link sign-in for the `integration` Playwright project
// (PRO-P32, root tasks.md).
//
// The golden-paths suite's mutating flows (`/persons/new`, `/edit`,
// `/merge`) go through the page-visit auth guard (PRO-H10) and the CSRF
// double-submit check (PRO-H5), both landed 2026-08-28/29 after this
// suite's original "6/9 pass" baseline was written. Both need a real
// signed-in session — not a stub, and not a bypass of the guard/CSRF
// check itself (a decision this task's own note flagged as
// security-relevant and needing sign-off; a real sign-in avoids the
// question entirely by not adding any test-only carve-out to production
// code).
//
// This drives an actual passwordless sign-in against a LIVE
// authentication-service, exactly as a real user would, with one
// necessary substitute for "open the email": the service has no SMTP
// configured in `development` mode, so it logs the magic-link URL to its
// own console instead (`src/controllers/auth.rs::deliver_magic_link`,
// gated to `Environment::Development` only — see SEC-A3,
// agents/share/security.md — so this ONLY works against
// examples/compose/authentication-dev.yml, never the production-mode
// stacks). Reading that log line stands in for reading the inbox.
//
// Playwright's own recommended "authenticate once" shape (a `setup`
// project the real project `dependencies` on — see playwright.config.ts)
// rather than a top-level `globalSetup`: a `globalSetup` hook runs for
// EVERY invocation regardless of `--project`, which would force the
// `smoke` project (deliberately "no service required") to depend on a
// live authentication-service too. Scoping this to a project dependency
// means only `pnpm test:integration` pays for it.

import { test as setup } from "@playwright/test";
import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../../../..");
const AUTH_COMPOSE_FILE = path.join(
  REPO_ROOT,
  "examples/compose/authentication-dev.yml",
);

const AUTH_API_URL = process.env.AUTH_API_URL ?? "http://localhost:5150";
export const STORAGE_STATE_PATH = path.join(
  __dirname,
  ".auth-storage-state.json",
);

// Deterministic test identity. `signup` treats an already-registered
// email as a sign-in (still issues a fresh magic link — see
// controllers/auth.rs), so re-running this setup against the same
// long-lived dev database is safe and idempotent.
const TEST_EMAIL = "e2e-golden-paths@example.test";
const TEST_NAME = "E2E Golden Paths";

// Bounded polling for the log line — the container needs a moment to
// flush its tracing output after the HTTP response returns.
const LOG_POLL_ATTEMPTS = 20;
const LOG_POLL_INTERVAL_MS = 500;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Current `authentication-service` container log, via the compose
 *  service name (not the container name — robust to how the compose
 *  provider actually names containers). */
function readAuthLog(): string {
  try {
    return execFileSync(
      "podman",
      ["compose", "-f", AUTH_COMPOSE_FILE, "logs", "authentication-service"],
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    );
  } catch {
    return "";
  }
}

/** Extract the most recent magic-link token issued to {@link TEST_EMAIL}.
 *  Matches on the token param rather than parsing the whole embedded URL
 *  (deliver_magic_link's `frontend` origin is a compose-file/env concern
 *  this test does not need to trust) — reconstructing the verify URL
 *  against our own known `frontendBase` is more robust than trusting the
 *  logged one to match. */
// The authentication-service binary's own tracing-subscriber ANSI-colours
// its output regardless of whether the pipe on this end is a TTY (verified
// live: `podman compose logs` still carries `\x1b[...m` codes through a
// plain pipe), which would otherwise split "magic link issued" and its
// fields apart from a naive substring/regex match.
function stripAnsi(s: string): string {
  // eslint-disable-next-line no-control-regex
  return s.replace(/\x1b\[[0-9;]*m/g, "");
}

function extractLatestToken(log: string, email: string): string | null {
  const escapedEmail = email.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const lineRe = new RegExp(`email=${escapedEmail}.*token=([A-Za-z0-9_-]+)`);
  let latest: string | null = null;
  for (const rawLine of log.split("\n")) {
    const line = stripAnsi(rawLine);
    if (!line.includes("magic link issued")) continue;
    const m = line.match(lineRe);
    if (m?.[1]) latest = m[1];
  }
  return latest;
}

async function requestMagicLinkToken(
  email: string,
  name: string,
  returnUrl: string,
): Promise<string> {
  const res = await fetch(`${AUTH_API_URL}/api/auth/signup`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ email, name, return_url: returnUrl }),
  });
  if (res.status === 429) {
    throw new Error(
      `authentication-service rate-limited /signup for ${email} (429) — ` +
        `the service's per-email issuance cap is 5 requests / 5 minutes ` +
        `(src/rate_limit.rs). Wait, or reset the dev database: ` +
        `podman compose -f examples/compose/authentication-dev.yml down -v`,
    );
  }
  if (!res.ok) {
    throw new Error(
      `authentication-service /signup returned ${res.status} — is ` +
        `examples/compose/authentication-dev.yml up? (AUTH_API_URL=${AUTH_API_URL})`,
    );
  }

  for (let attempt = 0; attempt < LOG_POLL_ATTEMPTS; attempt += 1) {
    const token = extractLatestToken(readAuthLog(), email);
    if (token) return token;
    await sleep(LOG_POLL_INTERVAL_MS);
  }
  throw new Error(
    `Timed out waiting for authentication-service to log a magic-link ` +
      `token for ${email}. Confirm the service is running with ` +
      `LOCO_ENV=development (examples/compose/authentication-dev.yml) — ` +
      `a production-mode container never logs the link (SEC-A3).`,
  );
}

setup(
  "authenticate via a real magic-link sign-in",
  async ({ page, baseURL }) => {
    const frontendBase = baseURL ?? "http://localhost:4173";
    const token = await requestMagicLinkToken(
      TEST_EMAIL,
      TEST_NAME,
      frontendBase,
    );

    // The real /verify page: exchanges the token server-side, sets the
    // session + CSRF cookies, redirects to "/". Same path a real user's
    // inbox click would take.
    await page.goto(
      `${frontendBase}/verify?token=${encodeURIComponent(token)}`,
    );
    await page.waitForURL(`${frontendBase}/`, { timeout: 10_000 });
    await page.context().storageState({ path: STORAGE_STATE_PATH });
  },
);
