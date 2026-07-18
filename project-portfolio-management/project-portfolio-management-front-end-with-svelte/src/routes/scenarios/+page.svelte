<!--
  Scenario planning (`/scenarios`, PPM-4): create what-if candidate
  portfolios (member pids + budget cap), evaluate them over live
  data, and commit the feasible winner.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import { PpmClient, money, type Scenario, type ScenarioEvaluation } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let scenarios = $state<Scenario[]>([]);
  let evaluations = $state<Record<string, ScenarioEvaluation>>({});
  let error = $state<string | null>(null);

  let name = $state("");
  let members = $state("");
  let cap = $state("");
  let currency = $state("GBP");

  async function refresh() {
    try {
      scenarios = await ppm.listScenarios();
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

  async function create(event: SubmitEvent) {
    event.preventDefault();
    const pids = members
      .split(/[\s,]+/)
      .map((value) => value.trim())
      .filter(Boolean);
    const capMinor = cap.trim() ? Math.round(Number(cap) * 100) : undefined;
    await act(() =>
      ppm.createScenario({
        name,
        work_item_pids: pids,
        ...(capMinor !== undefined && Number.isFinite(capMinor)
          ? { budget_cap_minor: capMinor, currency }
          : {}),
      }),
    );
    name = "";
    members = "";
    cap = "";
  }

  async function evaluate(pid: string) {
    try {
      evaluations = { ...evaluations, [pid]: await ppm.evaluateScenario(pid) };
    } catch (err) {
      error = err instanceof Error ? err.message : "evaluate failed";
    }
  }
</script>

<svelte:head><title>Scenarios — PPM</title></svelte:head>

<h1>{t("ppm.scenarios.title")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

<form class="row" onsubmit={create}>
  <input placeholder={t("ppm.scenarios.name")} bind:value={name} required />
  <input placeholder="work-item pids (space/comma separated)" bind:value={members} size="50" />
  <input placeholder={t("ppm.scenarios.cap")} bind:value={cap} size="12" />
  <input bind:value={currency} size="4" aria-label="Currency" />
  <button class="button primary" type="submit">{t("ppm.scenarios.create")}</button>
</form>

<table>
  <thead><tr><th>Scenario</th><th>Cap</th><th>Status</th><th>Evaluation</th><th></th></tr></thead>
  <tbody>
    {#each scenarios as scenario (scenario.pid)}
      <tr>
        <td><strong>{scenario.name}</strong></td>
        <td>
          {scenario.budget_cap_minor !== null
            ? money(scenario.budget_cap_minor, scenario.currency)
            : "—"}
        </td>
        <td><span class="chip">{scenario.status}</span></td>
        <td class="small">
          {#if evaluations[scenario.pid]}
            {@const evaluated = evaluations[scenario.pid]}
            {#if evaluated}
              {#each evaluated.evaluation.planned_by_currency as [code, total] (code)}
                <span class="chip">{code} {money(total)}</span>
              {/each}
              <span class="chip">{t("ppm.scenarios.exposure")} {evaluated.evaluation.total_exposure}</span>
              <span class="chip">{t("ppm.scenarios.alignment")} {evaluated.evaluation.total_alignment}</span>
              {#each evaluated.evaluation.violations as violation (violation)}
                <span class="chip red">{violation}</span>
              {/each}
              {#if evaluated.feasible}<span class="chip green">{t("ppm.scenarios.feasible")}</span>{/if}
            {/if}
          {/if}
        </td>
        <td>
          <button class="button small" onclick={() => evaluate(scenario.pid)}>{t("ppm.scenarios.evaluate")}</button>
          {#if scenario.status === "draft"}
            <button class="button primary small" onclick={() => act(() => ppm.commitScenario(scenario.pid))}>
              {t("ppm.scenarios.commit")}
            </button>
          {/if}
        </td>
      </tr>
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
    font-size: 0.78rem;
  }
  .chip.red { color: #a4262c; border-color: #a4262c; }
  .chip.green { color: #1d8a4e; border-color: #1d8a4e; }
</style>
