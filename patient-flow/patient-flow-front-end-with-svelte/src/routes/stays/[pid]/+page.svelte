<script lang="ts">
  // The stay detail page: journey facts, the Red2Green run, infection
  // flags, and the whiteboard actions (SAFER fields, red/green,
  // discharge). Every action calls an existing API mutation — no
  // board-only writes (spec `whiteboard.md`).
  import { invalidateAll } from "$app/navigation";
  import {
    addInfectionFlag,
    clearInfectionFlag,
    discharge,
    dischargeReady,
    recordRedGreen,
    transferStay,
    updateStay,
  } from "$lib/api/flow";

  let { data } = $props();
  let detail = $derived(data.detail);
  let stay = $derived(data.detail.stay);

  let error = $state<string | null>(null);
  let eddInput = $state("");
  let ccdMet = $state(false);
  let dayColor = $state<"red" | "green">("green");
  let delayReason = $state("");
  let pathway = $state("p0");
  let destination = $state("home");
  let toBed = $state("");
  let transferReason = $state("clinical");
  let organism = $state("");
  let precaution = $state("droplet");

  const DELAY_REASONS = [
    "",
    "awaiting_senior_review",
    "awaiting_diagnostics",
    "awaiting_pharmacy",
    "awaiting_transport",
    "awaiting_therapy_assessment",
    "awaiting_social_care",
    "awaiting_community_bed",
    "awaiting_care_package",
    "family_choice",
    "internal_process",
    "other",
  ];

  async function act(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
      await invalidateAll();
    } catch (e) {
      error = e instanceof Error ? e.message : "action failed";
    }
  }

  let alerts = $derived(Array.isArray(stay.alerts) ? stay.alerts : []);
</script>

<h1>{stay.display_name}</h1>
{#if error}<p class="error">{error}</p>{/if}

<div class="panel">
  <div class="chips">
    <span class="chip">{stay.status}</span>
    <span class="chip">from {stay.source}</span>
    <span class="chip">LOS {detail.length_of_stay_days}d</span>
    {#if detail.dtoc}<span class="chip danger">DTOC</span>{/if}
    {#if stay.edd}
      <span class="chip">EDD {stay.edd}</span>
    {:else}
      <span class="chip warn">EDD missing</span>
    {/if}
    {#if stay.ccd_met}<span class="chip ok">CCD met</span>{/if}
    {#if stay.discharge_pathway}
      <span class="chip">{stay.discharge_pathway.toUpperCase()}</span>
    {/if}
    {#each alerts as alert (alert)}<span class="chip warn">{alert}</span>{/each}
  </div>
  <p class="muted">
    {stay.person_ref}
    {#if stay.home_location_note}· {stay.home_location_note}{/if}
    {#if stay.ccd}· CCD: {stay.ccd}{/if}
  </p>
  <p>
    Red2Green:
    {#each detail.red_green as day (day.day)}
      <span
        class="chip {day.classification === 'red' ? 'red-day' : 'green-day'}"
        title={day.day}>{day.classification === "red" ? "R" : "G"}</span
      >
    {/each}
  </p>
  {#if detail.infection_flags.length > 0}
    <p>
      Flags:
      {#each detail.infection_flags as flag (flag.pid)}
        <span class="chip infection {flag.status}">
          {flag.organism ?? flag.precaution} ({flag.status})
          {#if !flag.cleared_at && stay.status !== "discharged"}
            <button
              style="margin-left:0.3rem"
              onclick={() => act(() => clearInfectionFlag(stay.pid, flag.pid))}
            >
              clear
            </button>
          {/if}
        </span>
      {/each}
    </p>
  {/if}
</div>

{#if stay.status !== "discharged"}
  <div class="panel">
    <h2>SAFER</h2>
    <form
      class="row"
      onsubmit={(e) => {
        e.preventDefault();
        act(() =>
          updateStay(stay.pid, {
            ...(eddInput ? { edd: eddInput } : {}),
            ccd_met: ccdMet,
          }),
        );
      }}
    >
      <label>EDD <input type="date" bind:value={eddInput} /></label>
      <label>
        <input type="checkbox" bind:checked={ccdMet} /> CCD met
      </label>
      <button type="submit">Save</button>
      <button
        type="button"
        onclick={() => act(() => updateStay(stay.pid, { senior_review_now: true }))}
      >
        Mark senior review
      </button>
    </form>

    <h2>Today's day</h2>
    <form
      class="row"
      onsubmit={(e) => {
        e.preventDefault();
        act(() =>
          recordRedGreen(
            stay.pid,
            dayColor,
            dayColor === "red" && delayReason ? [delayReason] : [],
          ),
        );
      }}
    >
      <select bind:value={dayColor}>
        <option value="green">green</option>
        <option value="red">red</option>
      </select>
      {#if dayColor === "red"}
        <select bind:value={delayReason}>
          {#each DELAY_REASONS as reason (reason)}
            <option value={reason}>{reason || "(no reason yet)"}</option>
          {/each}
        </select>
      {/if}
      <button type="submit">Record</button>
    </form>

    <h2>Infection flag</h2>
    <form
      class="row"
      onsubmit={(e) => {
        e.preventDefault();
        act(() =>
          addInfectionFlag(stay.pid, {
            precaution,
            organism: organism || null,
            status: "suspected",
          }),
        );
      }}
    >
      <select bind:value={precaution}>
        <option>contact</option>
        <option>droplet</option>
        <option>airborne</option>
        <option>protective</option>
      </select>
      <input placeholder="organism (e.g. covid-19)" bind:value={organism} />
      <button type="submit">Flag</button>
    </form>

    <h2>Transfer</h2>
    <form
      class="row"
      onsubmit={(e) => {
        e.preventDefault();
        act(() =>
          transferStay(stay.pid, { to_bed_pid: toBed, reason: transferReason }),
        );
      }}
    >
      <input placeholder="destination bed pid" bind:value={toBed} size="36" />
      <select bind:value={transferReason}>
        <option>clinical</option>
        <option>capacity</option>
        <option>isolation</option>
        <option>patient_request</option>
        <option>step_up</option>
        <option>step_down</option>
      </select>
      <button type="submit">Transfer</button>
    </form>

    <h2>Discharge</h2>
    <form class="row" onsubmit={(e) => e.preventDefault()}>
      {#if stay.status === "admitted"}
        <select bind:value={pathway}>
          <option value="p0">P0 — home</option>
          <option value="p1">P1 — home with support</option>
          <option value="p2">P2 — community bed</option>
          <option value="p3">P3 — 24h-care assessment</option>
        </select>
        <button onclick={() => act(() => dischargeReady(stay.pid, pathway))}>
          Mark discharge-ready
        </button>
      {/if}
      <select bind:value={destination}>
        <option>home</option>
        <option>home_with_support</option>
        <option>community_hospital</option>
        <option>care_home</option>
        <option>other_acute</option>
        <option>deceased</option>
        <option>self_discharge</option>
      </select>
      <button
        class="primary"
        onclick={() => act(() => discharge(stay.pid, destination))}
      >
        Discharge
      </button>
    </form>
  </div>
{/if}

<div class="panel">
  <h2>Moves</h2>
  <table>
    <thead>
      <tr><th>When</th><th>From</th><th>To</th><th>Reason</th></tr>
    </thead>
    <tbody>
      {#each detail.transfers as move (move.pid)}
        <tr>
          <td>{move.moved_at}</td>
          <td>{move.from_bed_pid ?? "—"}</td>
          <td>{move.to_bed_pid ?? "—"}</td>
          <td>{move.reason}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
