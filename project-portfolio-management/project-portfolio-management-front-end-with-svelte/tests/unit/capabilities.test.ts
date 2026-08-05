// CapabilityClient path mapping: the client is the single source of the
// collaboration / automation / prioritisation endpoint paths, so a wrong
// path or a dropped query parameter fails here first.

import { describe, expect, it, vi } from "vitest";
import { ApiClient } from "$lib/api/client";
import { CapabilityClient } from "$lib/api/capabilities";

/** A CapabilityClient whose fetch records every (url, method) pair. */
function recordingClient(): {
  api: CapabilityClient;
  calls: { url: string; method: string }[];
} {
  const calls: { url: string; method: string }[] = [];
  const fetchFn = vi.fn(
    async (input: RequestInfo | URL, init?: RequestInit) => {
      calls.push({ url: String(input), method: init?.method ?? "GET" });
      return new Response("{}", {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  ) as unknown as typeof fetch;
  const api = new CapabilityClient(
    new ApiClient({ baseUrl: "http://svc", fetch: fetchFn }),
  );
  return { api, calls };
}

describe("CapabilityClient paths", () => {
  it("maps the collaborative-review endpoints", async () => {
    const { api, calls } = recordingClient();
    await api.invite({
      subject_kind: "plan",
      subject_pid: "p1",
      reviewer_ref: "person:u1",
      reviewer_scope: "external",
    });
    await api.respond("r1", "accept");
    await api.submitVerdict("r1", { score: 80, recommendation: "advance" });
    await api.withdraw("r1");
    await api.consensus("plan", "p1");

    expect(calls.map((c) => `${c.method} ${c.url}`)).toEqual([
      "POST http://svc/api/reviews",
      "POST http://svc/api/reviews/r1/respond",
      "POST http://svc/api/reviews/r1/submit",
      "DELETE http://svc/api/reviews/r1",
      "GET http://svc/api/reviews/consensus?subject_kind=plan&subject_pid=p1",
    ]);
  });

  it("maps the automation + scheduler endpoints", async () => {
    const { api, calls } = recordingClient();
    await api.createAutomation({
      name: "Route reviews",
      trigger_kind: "task_moved",
      to_status: "in_review",
      action_kind: "assign",
      action_value: { assignee_ref: "person:u1" },
    });
    await api.setAutomationEnabled("a1", false);
    await api.setAutomationEnabled("a1", true);
    await api.deleteAutomation("a1");
    await api.runs({ outcome: "failed" });
    await api.schedule({
      subject_kind: "plan",
      subject_pid: "p1",
      action_kind: "expire_review",
      in_days: 14,
    });
    await api.cancelScheduled("s1");
    await api.sweep();

    expect(calls.map((c) => `${c.method} ${c.url}`)).toEqual([
      "POST http://svc/api/automations",
      "POST http://svc/api/automations/a1/disable",
      "POST http://svc/api/automations/a1/enable",
      "DELETE http://svc/api/automations/a1",
      "GET http://svc/api/automations/runs?outcome=failed",
      "POST http://svc/api/scheduled-actions",
      "DELETE http://svc/api/scheduled-actions/s1",
      "POST http://svc/api/scheduled-actions/sweep",
    ]);
  });

  it("maps the prioritisation + lifecycle endpoints", async () => {
    const { api, calls } = recordingClient();
    await api.prioritisation({ limit: 10, band: "high" });
    await api.smartScore("p1");
    await api.lifecycle();
    await api.planLifecycle("p1");
    await api.workload("p1");

    expect(calls.map((c) => c.url)).toEqual([
      "http://svc/api/prioritisation?limit=10&band=high",
      "http://svc/api/plans/p1/smart-score",
      "http://svc/api/lifecycle",
      "http://svc/api/plans/p1/lifecycle",
      "http://svc/api/assignees/workload?plan=p1",
    ]);
  });

  it("omits unset query parameters rather than sending blanks", async () => {
    const { api, calls } = recordingClient();
    await api.prioritisation();
    await api.listReviews();
    await api.listAutomations();
    await api.workload();
    await api.inbox("person:u1");

    expect(calls.map((c) => c.url)).toEqual([
      "http://svc/api/prioritisation",
      "http://svc/api/reviews",
      "http://svc/api/automations",
      "http://svc/api/assignees/workload",
      "http://svc/api/notifications?recipient=person%3Au1",
    ]);
  });

  it("sends the unread filter only when asked for", async () => {
    const { api, calls } = recordingClient();
    await api.inbox("person:u1", true);
    expect(calls[0]?.url).toBe(
      "http://svc/api/notifications?recipient=person%3Au1&unread=true",
    );
  });

  it("passes a null assignee through to unassign", async () => {
    const { api, calls } = recordingClient();
    await api.assign("p1", "t1", null);
    expect(calls[0]).toEqual({
      url: "http://svc/api/plans/p1/tasks/t1/assign",
      method: "POST",
    });
  });
});

describe("CapabilityClient pagination (agents/share/restful.md)", () => {
  /** A CapabilityClient whose fetch answers with `body` and `headers`. */
  function pagedClient(body: unknown, headers: Record<string, string>) {
    const fetchFn = (async () =>
      new Response(JSON.stringify(body), {
        status: 200,
        headers,
      })) as unknown as typeof fetch;
    return new CapabilityClient(
      new ApiClient({ baseUrl: "http://svc", fetch: fetchFn }),
    );
  }

  it("paginates the delegation list and carries the filter through", async () => {
    const calls: string[] = [];
    const fetchFn = (async (input: RequestInfo | URL) => {
      calls.push(String(input));
      return new Response(JSON.stringify([{ pid: "r1" }]), {
        status: 200,
        headers: { "x-total-count": "9", "x-limit": "2", "x-offset": "1" },
      });
    }) as unknown as typeof fetch;
    const api = new CapabilityClient(
      new ApiClient({ baseUrl: "http://svc", fetch: fetchFn }),
    );

    const page = await api.listReviewsPage(
      { status: "invited" },
      { limit: 2, offset: 1 },
    );

    expect(calls).toEqual([
      "http://svc/api/reviews?status=invited&limit=2&offset=1",
    ]);
    expect(page.items).toHaveLength(1);
    expect(page.total).toBe(9);
    expect(page.limit).toBe(2);
    expect(page.offset).toBe(1);
  });

  it("paginates the automation rules, run log, and deadline queue", async () => {
    const api = pagedClient([{ pid: "a1" }, { pid: "a2" }], {
      "x-total-count": "5",
      "x-limit": "2",
      "x-offset": "0",
    });

    const rules = await api.listAutomationsPage();
    expect(rules.items).toHaveLength(2);
    expect(rules.total).toBe(5);

    const runs = await api.runsPage();
    expect(runs.total).toBe(5);

    const queue = await api.listScheduledPage({ status: "pending" });
    expect(queue.total).toBe(5);
  });

  it("paginates one recipient's notification inbox", async () => {
    const api = pagedClient([{ pid: "n1" }], {
      "x-total-count": "3",
      "x-limit": "1",
      "x-offset": "0",
    });
    const page = await api.inboxPage("person:u1", true, { limit: 1 });
    expect(page.items).toHaveLength(1);
    expect(page.total).toBe(3);
  });
});
