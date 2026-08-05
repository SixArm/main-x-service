<!--
  Collaborative review (`/reviews`): delegate an idea, proposal, or plan
  to the right internal or external expert, then track who accepted, who
  declined, and who still owes a verdict.

  Reviewers are named by EntityRef URN (person:/worker:/organization:),
  never by raw email — an outside expert is a record in the person
  registry, so no contact detail is duplicated here. The internal /
  external scope is chosen explicitly, because sharing something with an
  outsider is a disclosure decision, not an inference.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    CapabilityClient,
    type Consensus,
    type Recommendation,
    type Review,
    type ReviewStatus,
    type ReviewSubjectKind,
    type ReviewerScope,
  } from "$lib/api/capabilities";

  const api = CapabilityClient.withFetch();
  let reviews = $state<Review[]>([]);
  // Rows matching overall, from `X-Total-Count` — not `reviews.length`,
  // which is only this page (agents/share/restful.md).
  let reviewsTotal = $state(0);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let statusFilter = $state<ReviewStatus | "">("");

  // Delegation form.
  let subjectKind = $state<ReviewSubjectKind>("plan");
  let subjectPid = $state("");
  let reviewerRef = $state("");
  let reviewerScope = $state<ReviewerScope>("internal");
  let expertise = $state("");
  let dueOn = $state("");

  // Consensus panel.
  let consensusFor = $state<string | null>(null);
  let consensus = $state<Consensus | null>(null);

  async function refresh() {
    try {
      const page = await api.listReviewsPage({
        status: statusFilter || undefined,
      });
      reviews = page.items;
      reviewsTotal = page.total;
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : "Could not load reviews";
    }
  }

  async function delegate(event: SubmitEvent) {
    event.preventDefault();
    try {
      await api.invite({
        subject_kind: subjectKind,
        subject_pid: subjectPid.trim(),
        reviewer_ref: reviewerRef.trim(),
        reviewer_scope: reviewerScope,
        expertise: expertise.trim() || undefined,
        due_on: dueOn || undefined,
      });
      notice = "Invitation sent";
      error = null;
      reviewerRef = "";
      expertise = "";
      await refresh();
    } catch (err) {
      notice = null;
      error = err instanceof Error ? err.message : "Could not delegate";
    }
  }

  async function respond(pid: string, response: "accept" | "decline") {
    await act(() => api.respond(pid, response));
  }

  async function submitVerdict(pid: string, recommendation: Recommendation) {
    const raw = window.prompt("Score 0–100 (leave blank for none)") ?? "";
    const score = raw.trim() === "" ? undefined : Number(raw);
    if (score !== undefined && (Number.isNaN(score) || score < 0 || score > 100)) {
      error = "Score must be between 0 and 100";
      return;
    }
    await act(() => api.submitVerdict(pid, { score, recommendation }));
  }

  async function withdraw(pid: string) {
    await act(() => api.withdraw(pid));
  }

  async function act(run: () => Promise<unknown>) {
    try {
      await run();
      error = null;
      await refresh();
      if (consensusFor) await showConsensus(consensusFor);
    } catch (err) {
      error = err instanceof Error ? err.message : "That action was refused";
    }
  }

  async function showConsensus(pid: string) {
    consensusFor = pid;
    try {
      consensus = await api.consensus(subjectKind, pid);
    } catch (err) {
      error = err instanceof Error ? err.message : "Could not load consensus";
    }
  }

  onMount(refresh);
</script>

<svelte:head><title>Reviews — PPM</title></svelte:head>

<h1>Collaborative review</h1>
<p class="small muted">
  Delegate a specific item to the right expert. Only a reviewer who
  accepted can submit a verdict, so an unanswered invitation never
  becomes evidence.
</p>

{#if error}<p class="banner" role="alert">{error}</p>{/if}
{#if notice}<p class="small">{notice}</p>{/if}

<form class="grid" onsubmit={delegate}>
  <label class="small">
    subject
    <select bind:value={subjectKind}>
      <option value="plan">plan</option>
      <option value="proposal">proposal</option>
      <option value="idea">idea</option>
    </select>
  </label>
  <label class="small">
    subject pid
    <input bind:value={subjectPid} placeholder="uuid" required />
  </label>
  <label class="small">
    reviewer
    <input bind:value={reviewerRef} placeholder="person:&lt;uuid&gt;" required />
  </label>
  <label class="small">
    scope
    <select bind:value={reviewerScope}>
      <option value="internal">internal</option>
      <option value="external">external</option>
    </select>
  </label>
  <label class="small">
    expertise
    <input bind:value={expertise} placeholder="e.g. clinical safety" />
  </label>
  <label class="small">due <input type="date" bind:value={dueOn} /></label>
  <button class="button small" type="submit">Delegate</button>
</form>

<div class="row">
  <label class="small">
    status
    <select bind:value={statusFilter} onchange={refresh}>
      <option value="">all</option>
      <option value="invited">invited</option>
      <option value="accepted">accepted</option>
      <option value="submitted">submitted</option>
      <option value="declined">declined</option>
      <option value="expired">expired</option>
    </select>
  </label>
  {#if subjectPid}
    <button class="button small" type="button" onclick={() => showConsensus(subjectPid.trim())}>
      Consensus for this subject
    </button>
  {/if}
</div>

{#if consensus}
  <div class="panel">
    <strong>Consensus</strong>
    <p class="small">
      {consensus.submitted} submitted · {consensus.outstanding} outstanding ·
      {consensus.declined} declined ·
      {consensus.external_submitted} external
    </p>
    <p class="small">
      mean score: {consensus.mean_score ?? "—"} · majority:
      {consensus.majority ?? "no majority"} ·
      {consensus.complete ? "everyone has answered" : "still waiting"}
    </p>
  </div>
{/if}

{#if reviews.length > 0}
  <p class="count">
    {reviews.length === reviewsTotal
      ? `${reviewsTotal}`
      : `${reviews.length} / ${reviewsTotal}`}
  </p>
{/if}
<table>
  <thead>
    <tr>
      <th>Subject</th><th>Reviewer</th><th>Scope</th><th>Expertise</th>
      <th>Status</th><th>Verdict</th><th>Actions</th>
    </tr>
  </thead>
  <tbody>
    {#each reviews as review (review.pid)}
      <tr>
        <td class="small">{review.subject_kind} {review.subject_pid.slice(0, 8)}…</td>
        <td class="small">{review.reviewer_ref}</td>
        <td>
          <span class="chip" class:external={review.reviewer_scope === "external"}>
            {review.reviewer_scope}
          </span>
        </td>
        <td class="small muted">{review.expertise ?? "—"}</td>
        <td>{review.status}</td>
        <td class="small">
          {#if review.status === "submitted"}
            {review.recommendation} · {review.score ?? "no score"}
          {:else}
            —
          {/if}
        </td>
        <td class="actions">
          {#if review.status === "invited"}
            <button class="button small" type="button" onclick={() => respond(review.pid, "accept")}>
              Accept
            </button>
            <button class="button small" type="button" onclick={() => respond(review.pid, "decline")}>
              Decline
            </button>
          {:else if review.status === "accepted"}
            <button class="button small" type="button" onclick={() => submitVerdict(review.pid, "advance")}>
              Advance
            </button>
            <button class="button small" type="button" onclick={() => submitVerdict(review.pid, "hold")}>
              Hold
            </button>
            <button class="button small" type="button" onclick={() => submitVerdict(review.pid, "reject")}>
              Reject
            </button>
          {/if}
          {#if review.status === "invited" || review.status === "accepted"}
            <button class="button small" type="button" onclick={() => withdraw(review.pid)}>
              Withdraw
            </button>
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
{#if reviews.length === 0}
  <p class="small muted">No review invitations yet.</p>
{/if}

<style>
  .grid {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: flex-end;
    margin: 0.8rem 0;
  }
  .row { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; margin: 0.8rem 0; }
  .actions { display: flex; gap: 0.25rem; flex-wrap: wrap; }
  .panel {
    border: 1px solid var(--border, #ddd);
    border-radius: 6px;
    padding: 0.6rem 0.8rem;
    margin: 0.8rem 0;
  }
  .chip {
    display: inline-block;
    border: 1px solid currentColor;
    border-radius: 999px;
    padding: 0 0.5rem;
    font-size: 0.75rem;
  }
  .chip.external { color: #8a6d1d; font-weight: 700; }

  /* The row count: just the total when the page holds everything, and
     "shown / total" when it does not, so a first page of a long list
     cannot be mistaken for a short list (same convention as the
     organization service's front-end /organizations list route). */
  .count {
    margin: 0 0 0.3rem;
    opacity: 0.75;
    font-variant-numeric: tabular-nums;
    font-size: 0.85rem;
  }
</style>
