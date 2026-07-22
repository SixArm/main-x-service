<!--
  Work-intake board (`/proposals`, PPM-1): the pipeline (draft →
  submitted → in_review → approved/rejected → promoted) with a
  new-proposal form, per-row pipeline actions, matcher-backed
  duplicate-demand checks, and promote-to-work-item.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import { PpmClient, type DemandHit, type Proposal, money } from "$lib/api/ppm";
  import { COLLECTIONS, type Collection } from "$lib/api/types";

  const ppm = PpmClient.withFetch();
  let proposals = $state<Proposal[]>([]);
  let statusFilter = $state<string>("");
  let error = $state<string | null>(null);
  let duplicates = $state<Record<string, DemandHit[]>>({});

  let title = $state("");
  let summary = $state("");
  let kindTarget = $state<Collection>("projects");
  let requested = $state("");
  let currency = $state("GBP");

  async function refresh() {
    try {
      proposals = await ppm.listProposals(statusFilter || undefined);
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  }

  onMount(refresh);

  async function act(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
      await refresh();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.actionFailed");
    }
  }

  async function submitNew(event: SubmitEvent) {
    event.preventDefault();
    const minor = requested.trim() ? Math.round(Number(requested) * 100) : undefined;
    await act(() =>
      ppm.createProposal({
        title,
        summary: summary || undefined,
        kind_target: kindTarget,
        ...(minor !== undefined && Number.isFinite(minor)
          ? { requested_minor: minor, currency }
          : {}),
      }),
    );
    title = "";
    summary = "";
    requested = "";
  }

  async function showDuplicates(pid: string) {
    try {
      duplicates = { ...duplicates, [pid]: await ppm.proposalDuplicates(pid) };
    } catch (err) {
      error = err instanceof Error ? err.message : "duplicate check failed";
    }
  }

  /** The pipeline actions legal from a status. */
  function actionsFor(status: Proposal["status"]): ("submit" | "review" | "approve" | "reject")[] {
    if (status === "draft") return ["submit"];
    if (status === "submitted") return ["review"];
    if (status === "in_review") return ["approve", "reject"];
    return [];
  }
</script>

<svelte:head><title>Proposals — PPM</title></svelte:head>

<h1>{t("ppm.proposals.title")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

<form class="row" onsubmit={submitNew}>
  <input placeholder={t("ppm.proposals.formTitle")} bind:value={title} required />
  <input placeholder={t("ppm.proposals.formSummary")} bind:value={summary} />
  <select bind:value={kindTarget} aria-label="Target collection">
    {#each COLLECTIONS as target (target)}<option value={target}>{target}</option>{/each}
  </select>
  <input placeholder={t("ppm.proposals.requested")} bind:value={requested} size="12" />
  <input bind:value={currency} size="4" aria-label="Currency" />
  <button class="button primary" type="submit">{t("ppm.proposals.open")}</button>
</form>

<p class="row small">
  {t("ppm.proposals.filter")}
  <select bind:value={statusFilter} onchange={refresh} aria-label="Status filter">
    <option value="">{t("ppm.proposals.allOpen")}</option>
    {#each ["draft", "submitted", "in_review", "approved", "rejected", "promoted"] as status (status)}
      <option value={status}>{status}</option>
    {/each}
  </select>
</p>

<table>
  <thead>
    <tr><th>Title</th><th>Target</th><th>Requested</th><th>Status</th><th>Actions</th></tr>
  </thead>
  <tbody>
    {#each proposals as proposal (proposal.pid)}
      <tr>
        <td>
          <strong>{proposal.title}</strong>
          {#if proposal.summary}<div class="small muted">{proposal.summary}</div>{/if}
        </td>
        <td>{proposal.kind_target}</td>
        <td>
          {proposal.requested_minor !== null
            ? money(proposal.requested_minor, proposal.currency)
            : "—"}
        </td>
        <td><span class="chip">{proposal.status}</span></td>
        <td>
          {#each actionsFor(proposal.status) as action (action)}
            <button class="button small" onclick={() => act(() => ppm.proposalAction(proposal.pid, action))}>
              {action}
            </button>
          {/each}
          {#if proposal.status === "approved"}
            <button class="button primary small" onclick={() => act(() => ppm.promoteProposal(proposal.pid))}>
              promote
            </button>
          {/if}
          {#if proposal.status === "promoted" && proposal.promoted_plan_pid}
            <a href={`/plans/${proposal.promoted_plan_pid}`}>{t("ppm.proposals.workItem")}</a>
          {:else}
            <button class="button small" onclick={() => showDuplicates(proposal.pid)}>
              {t("ppm.proposals.duplicates")}
            </button>
          {/if}
        </td>
      </tr>
      {#if duplicates[proposal.pid]}
        <tr>
          <td colspan="5" class="small">
            {#if (duplicates[proposal.pid] ?? []).length === 0}
              {t("ppm.proposals.noDuplicates")}
            {:else}
              {#each duplicates[proposal.pid] ?? [] as hit (hit.pid)}
                <span class="chip">
                  {hit.source === "plan" ? "live:" : "proposal:"}
                  {hit.name} ({hit.score.toFixed(2)})
                </span>
              {/each}
            {/if}
          </td>
        </tr>
      {/if}
    {/each}
  </tbody>
</table>

<style>
  .row { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; margin: 0.8rem 0; }
  .chip {
    display: inline-block;
    border: 1px solid var(--border, #ccc);
    border-radius: 999px;
    padding: 0 0.55rem;
    margin: 0.1rem;
    font-size: 0.8rem;
  }
</style>
