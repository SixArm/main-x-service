<!--
  Governance panel (`/[collection]/[pid]/governance`): the PPM view of
  one work item — the summary strip (stage / risk posture / budget
  variance), the gate journey, risks, budget lines, benefits + ROI,
  OKR mappings, milestones, and allocations, each with its minimal
  add-form. Every action maps 1:1 to a service endpoint.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import {
    PpmClient,
    money,
    type Allocation,
    type BenefitBoard,
    type BudgetBoard,
    type GateJourney,
    type GovernanceSummary,
    type Milestone,
    type Objective,
    type Risk,
  } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  const pid = $derived(page.params.pid ?? "");

  let summary = $state<GovernanceSummary | null>(null);
  let journey = $state<GateJourney | null>(null);
  let risks = $state<Risk[]>([]);
  let budget = $state<BudgetBoard | null>(null);
  let benefits = $state<BenefitBoard | null>(null);
  let objectives = $state<Objective[]>([]);
  let mappings = $state<{ objective_pid: string; title: string; weight: number }[]>([]);
  let milestones = $state<Milestone[]>([]);
  let allocations = $state<Allocation[]>([]);
  let error = $state<string | null>(null);

  // Add-form state (one small form per section).
  let gateDecision = $state("approved");
  let riskTitle = $state("");
  let riskProbability = $state(3);
  let riskImpact = $state(3);
  let lineDescription = $state("");
  let lineCategory = $state("capex");
  let lineCurrency = $state("GBP");
  let linePlanned = $state("");
  let benefitTitle = $state("");
  let benefitCategory = $state("cost_saving");
  let benefitTarget = $state("");
  let mapObjective = $state("");
  let mapWeight = $state(3);
  let milestoneName = $state("");
  let milestoneDue = $state("");
  let allocPerson = $state("");
  let allocPercent = $state(50);

  async function refresh() {
    try {
      [summary, journey, risks, budget, benefits, mappings, milestones, allocations, objectives] =
        await Promise.all([
          ppm.governance(pid),
          ppm.gateJourney(pid),
          ppm.listRisks(pid),
          ppm.budget(pid),
          ppm.benefits(pid),
          ppm.itemObjectives(pid),
          ppm.milestones(pid),
          ppm.allocations(pid),
          ppm.listObjectives(),
        ]);
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

  const toMinor = (major: string) => Math.round(Number(major) * 100);
</script>

<svelte:head><title>Governance — PPM</title></svelte:head>

<p class="small"><a href={`/plans/`}>{t("ppm.gov.backToItem")}</a></p>
<h1>{t("ppm.gov.title")}{#if summary} — {summary.name}{/if}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if summary}
  <section class="strip" data-testid="governance-strip">
    <span class="chip">stage: {summary.stage ?? "pre-gate"}</span>
    {#if summary.next_gate}<span class="chip">next: {summary.next_gate}</span>{/if}
    <span class="chip">open risks: {summary.risks.open}</span>
    {#if summary.risks.materialised > 0}
      <span class="chip red">materialised: {summary.risks.materialised}</span>
    {/if}
    {#if summary.risks.max_exposure !== null}
      <span class="chip">max exposure: {summary.risks.max_exposure}</span>
    {/if}
    {#each summary.budget as total (total.currency)}
      <span class="chip" class:red={total.variance_minor < 0}>
        {total.currency}: {money(total.actual_minor)} / {money(total.planned_minor)}
      </span>
    {/each}
    <a class="button small" href={`/plans/${pid}/schedule`}>{t("ppm.common.schedule")}</a>
  </section>
{/if}

<details open>
  <summary><h2>{t("ppm.gov.gates")}</h2></summary>
  {#if journey}
    <p class="small">
      {#each journey.reviews as review (review.pid)}
        <span class="chip" class:green={review.decision.startsWith("approved")}>
          {review.gate}: {review.decision}
        </span>
      {/each}
    </p>
    {#if journey.next_gate}
      <form
        class="row"
        onsubmit={(e) => {
          e.preventDefault();
          act(() =>
            ppm.reviewGate(pid, {
              gate: journey?.next_gate,
              decision: gateDecision,
            }),
          );
        }}
      >
        <span class="small">Review <strong>{journey.next_gate}</strong>:</span>
        <select bind:value={gateDecision} aria-label="Decision">
          <option>approved</option>
          <option>approved_with_conditions</option>
          <option>hold</option>
          <option>rejected</option>
        </select>
        <button class="button primary small" type="submit">{t("ppm.gov.recordDecision")}</button>
      </form>
    {:else}
      <p class="small muted">{t("ppm.gov.journeyComplete")}</p>
    {/if}
  {/if}
</details>

<details open>
  <summary><h2>{t("ppm.gov.risks")}</h2></summary>
  <table>
    <thead><tr><th>Risk</th><th>P×I</th><th>Status</th><th></th></tr></thead>
    <tbody>
      {#each risks as risk (risk.pid)}
        <tr>
          <td>{risk.title}{#if risk.mitigation}<div class="small muted">{risk.mitigation}</div>{/if}</td>
          <td>{risk.probability}×{risk.impact} = <strong>{risk.exposure}</strong></td>
          <td><span class="chip" class:red={risk.status === "materialised"}>{risk.status}</span></td>
          <td>
            {#if risk.status === "open" || risk.status === "mitigating"}
              <button class="button danger small" onclick={() => act(() => ppm.escalateRisk(pid, risk.pid))}>
                {t("ppm.gov.escalate")}
              </button>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      act(() =>
        ppm.createRisk(pid, {
          title: riskTitle,
          probability: riskProbability,
          impact: riskImpact,
        }),
      ).then(() => (riskTitle = ""));
    }}
  >
    <input placeholder="New risk" bind:value={riskTitle} required />
    <label class="small">P <input type="number" min="1" max="5" bind:value={riskProbability} /></label>
    <label class="small">I <input type="number" min="1" max="5" bind:value={riskImpact} /></label>
    <button class="button small" type="submit">{t("ppm.gov.raise")}</button>
  </form>
</details>

<details open>
  <summary><h2>{t("ppm.gov.budget")}</h2></summary>
  {#if budget}
    <table>
      <thead><tr><th>Line</th><th>Planned</th><th>Actual</th><th>Record actual</th></tr></thead>
      <tbody>
        {#each budget.lines as line (line.pid)}
          <tr>
            <td>{line.description} <span class="chip">{line.category}</span></td>
            <td>{money(line.planned_minor, line.currency)}</td>
            <td>{money(line.actual_minor, line.currency)}</td>
            <td>
              <form
                class="row"
                onsubmit={(e) => {
                  e.preventDefault();
                  const input = (e.currentTarget as HTMLFormElement).elements.namedItem(
                    "amount",
                  ) as HTMLInputElement;
                  const minor = toMinor(input.value);
                  if (Number.isFinite(minor)) {
                    act(() => ppm.recordActual(pid, line.pid, minor));
                    input.value = "";
                  }
                }}
              >
                <input name="amount" size="8" placeholder="0.00" />
                <button class="button small" type="submit">+</button>
              </form>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#each budget.totals as total (total.currency)}
      <p class="small">
        <strong>{total.currency}</strong>: planned {money(total.planned_minor)} · actual
        {money(total.actual_minor)} · variance {money(total.variance_minor)}
      </p>
    {/each}
  {/if}
  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      const minor = toMinor(linePlanned);
      if (!Number.isFinite(minor)) return;
      act(() =>
        ppm.createBudgetLine(pid, {
          category: lineCategory,
          description: lineDescription,
          currency: lineCurrency,
          planned_minor: minor,
        }),
      ).then(() => {
        lineDescription = "";
        linePlanned = "";
      });
    }}
  >
    <input placeholder="New budget line" bind:value={lineDescription} required />
    <select bind:value={lineCategory} aria-label="Category"><option>capex</option><option>opex</option></select>
    <input bind:value={lineCurrency} size="4" aria-label="Currency" />
    <input placeholder="Planned (major)" bind:value={linePlanned} size="10" required />
    <button class="button small" type="submit">{t("ppm.gov.addLine")}</button>
  </form>
</details>

<details open>
  <summary><h2>{t("ppm.gov.benefits")}</h2></summary>
  {#if benefits}
    <table>
      <thead><tr><th>Benefit</th><th>Target</th><th>Realized</th><th>Status</th><th></th></tr></thead>
      <tbody>
        {#each benefits.benefits as benefit (benefit.pid)}
          <tr>
            <td>{benefit.title} <span class="chip">{benefit.category}</span></td>
            <td>
              {benefit.target_minor !== null
                ? money(benefit.target_minor, benefit.currency)
                : (benefit.target_note ?? "—")}
            </td>
            <td>{money(benefit.realized_minor, benefit.currency)}</td>
            <td><span class="chip">{benefit.status}</span></td>
            <td>
              <form
                class="row"
                onsubmit={(e) => {
                  e.preventDefault();
                  const input = (e.currentTarget as HTMLFormElement).elements.namedItem(
                    "realize",
                  ) as HTMLInputElement;
                  const minor = toMinor(input.value);
                  if (Number.isFinite(minor)) {
                    act(() =>
                      ppm.realizeBenefit(pid, benefit.pid, {
                        amount_minor: minor,
                        status: "on_track",
                      }),
                    );
                    input.value = "";
                  }
                }}
              >
                <input name="realize" size="8" placeholder="0.00" />
                <button class="button small" type="submit">{t("ppm.gov.realize")}</button>
              </form>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#each benefits.totals as total (total.currency)}
      <p class="small">
        <strong>{total.currency}</strong>: target {money(total.target_minor)} · realized
        {money(total.realized_minor)} · spend {money(total.spend_minor)}
        {#if total.roi_basis_points !== null}
          · ROI {(total.roi_basis_points / 100).toFixed(1)}%
        {/if}
      </p>
    {/each}
  {/if}
  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      const minor = benefitTarget.trim() ? toMinor(benefitTarget) : undefined;
      act(() =>
        ppm.createBenefit(pid, {
          title: benefitTitle,
          category: benefitCategory,
          ...(minor !== undefined && Number.isFinite(minor)
            ? { currency: lineCurrency, target_minor: minor }
            : { target_note: "qualitative" }),
        }),
      ).then(() => {
        benefitTitle = "";
        benefitTarget = "";
      });
    }}
  >
    <input placeholder="New benefit" bind:value={benefitTitle} required />
    <select bind:value={benefitCategory} aria-label="Benefit category">
      <option>cost_saving</option><option>revenue</option><option>risk_reduction</option>
      <option>quality</option><option>compliance</option><option>other</option>
    </select>
    <input placeholder="Target (major)" bind:value={benefitTarget} size="10" />
    <button class="button small" type="submit">{t("ppm.gov.declare")}</button>
  </form>
</details>

<details>
  <summary><h2>{t("ppm.gov.objectives")}</h2></summary>
  <p class="small">
    {#each mappings as mapping (mapping.objective_pid)}
      <span class="chip">{mapping.title} (w{mapping.weight})</span>
    {/each}
    {#if mappings.length === 0}<span class="muted">{t("ppm.gov.noMappings")}</span>{/if}
  </p>
  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      if (mapObjective) act(() => ppm.linkObjective(pid, mapObjective, mapWeight));
    }}
  >
    <select bind:value={mapObjective} aria-label="Objective">
      <option value="">choose objective…</option>
      {#each objectives as objective (objective.pid)}
        <option value={objective.pid}>{objective.title}</option>
      {/each}
    </select>
    <label class="small">weight <input type="number" min="1" max="5" bind:value={mapWeight} /></label>
    <button class="button small" type="submit">{t("ppm.gov.map")}</button>
  </form>
</details>

<details>
  <summary><h2>{t("ppm.gov.milestones")}</h2></summary>
  <ul>
    {#each milestones as milestone (milestone.pid)}
      <li class="small">
        {milestone.name} — {milestone.due}
        {#if milestone.done}<span class="chip green">{t("ppm.gov.done")}</span>
        {:else if milestone.overdue}<span class="chip red">{t("ppm.gov.overdue")}</span>
        {:else}
          <button class="button small" onclick={() => act(() => ppm.completeMilestone(pid, milestone.pid))}>
            {t("ppm.gov.complete")}
          </button>
        {/if}
      </li>
    {/each}
  </ul>
  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      act(() => ppm.createMilestone(pid, { name: milestoneName, due: milestoneDue })).then(
        () => (milestoneName = ""),
      );
    }}
  >
    <input placeholder="New milestone" bind:value={milestoneName} required />
    <input type="date" bind:value={milestoneDue} required />
    <button class="button small" type="submit">{t("ppm.gov.add")}</button>
  </form>
</details>

<details>
  <summary><h2>{t("ppm.gov.allocations")}</h2></summary>
  <ul>
    {#each allocations as allocation (allocation.pid)}
      <li class="small">
        {allocation.person_ref} — {allocation.percent}%
        {#if allocation.role}({allocation.role}){/if}
      </li>
    {/each}
  </ul>
  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      act(() =>
        ppm.createAllocation(pid, {
          person_ref: allocPerson,
          percent: allocPercent,
        }),
      ).then(() => (allocPerson = ""));
    }}
  >
    <input placeholder="worker:<uuid>" bind:value={allocPerson} size="42" required />
    <label class="small">% <input type="number" min="1" max="100" bind:value={allocPercent} /></label>
    <button class="button small" type="submit">{t("ppm.gov.allocate")}</button>
  </form>
</details>

<style>
  .strip { display: flex; gap: 0.4rem; flex-wrap: wrap; align-items: center; margin: 0.6rem 0 1.2rem; }
  .row { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; margin: 0.5rem 0; }
  .chip {
    display: inline-block;
    border: 1px solid var(--border, #ccc);
    border-radius: 999px;
    padding: 0 0.55rem;
    font-size: 0.8rem;
  }
  .chip.red { color: #a4262c; border-color: #a4262c; }
  .chip.green { color: #1d8a4e; border-color: #1d8a4e; }
  details > summary { cursor: pointer; }
  details > summary h2 { display: inline; font-size: 1.05rem; }
  input[type="number"] { width: 4rem; }
</style>
