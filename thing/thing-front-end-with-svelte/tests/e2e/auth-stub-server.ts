// A minimal in-process stub of the authentication service's magic-link
// endpoints, for the /signin and /verify Playwright specs (T-23).
//
// The SvelteKit server's own outbound fetch calls (src/lib/server/auth.ts)
// happen in Node, not the browser — `page.route` can only intercept
// browser-initiated requests, so it cannot stub these. This stub instead
// runs as a real HTTP server the built preview server talks to, pointed at
// via `AUTH_API_URL` (see playwright.config.ts's `webServer.env`).
//
// Three fixed tokens drive the three `/verify` scenarios the real
// authentication service can produce:
//   - "valid-token-123"    → 200 + Set-Cookie (the happy path)
//   - "expired-token-456"  → 401 (a real, reachable rejection)
//   - "network-error-token" → the connection is reset mid-request,
//     simulating the service being unreachable — a different failure
//     class from a 401 (`fetch` throws rather than resolving).
// Any other token also 401s, matching the "unknown token" case.

import { createServer, type Server } from "node:http";

const VALID_TOKEN = "valid-token-123";
const EXPIRED_TOKEN = "expired-token-456";
const NETWORK_ERROR_TOKEN = "network-error-token";

export { VALID_TOKEN, EXPIRED_TOKEN, NETWORK_ERROR_TOKEN };

/** Start the stub on `port`. Returns the listening server. */
export function startAuthStub(port: number): Promise<Server> {
  const server = createServer((req, res) => {
    if (req.method === "POST" && req.url === "/api/auth/magic-link") {
      res.writeHead(200, { "content-type": "application/json" });
      res.end("{}");
      return;
    }
    const match = req.url?.match(/^\/api\/auth\/magic-link\/(.+)$/);
    if (req.method === "GET" && match) {
      const token = decodeURIComponent(match[1] ?? "");
      if (token === NETWORK_ERROR_TOKEN) {
        // Simulate the service being unreachable: reset the connection
        // instead of answering, so the client's `fetch` rejects rather
        // than resolving with a non-ok response.
        req.socket.destroy();
        return;
      }
      if (token === VALID_TOKEN) {
        res.writeHead(200, {
          "content-type": "application/json",
          "set-cookie": "__Host-mxi_session=stub-session-abc; Path=/",
        });
        res.end("{}");
        return;
      }
      // EXPIRED_TOKEN and anything else: a real, reachable rejection.
      res.writeHead(401, { "content-type": "application/json" });
      res.end("{}");
      return;
    }
    res.writeHead(404);
    res.end();
  });
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, "127.0.0.1", () => resolve(server));
  });
}
