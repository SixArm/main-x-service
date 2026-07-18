<script lang="ts">
  // The demand queue: open requests (priority then wait, each with its
  // live eligible-bed count), the ranked eligible list, allocation,
  // and a new-request form. The allocator advises; the operator picks
  // (PF-D7).
  import { invalidateAll } from "$app/navigation";
  import {
    allocateBed,
    cancelBedRequest,
    createBedRequest,
    getEligibleBeds,
  } from "$lib/api/flow";
  import type { EligibleBed } from "$lib/api/types";

  let { data } = $props();

  let error = $state<string | null>(null);
  let eligible = $state<Record<string, EligibleBed[]>>({});
  let personRef = $state("");
  let origin = $state("ed");
  let priority = $state<"emergency" | "urgent" | "routine">("urgent");
  let targetWard = $state("");
  let needsIsolation = $state(false);
  let sex = $state("");

  async function act(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
      await invalidateAll();
    } catch (e) {
      error = e instanceof Error ? e.message : "action failed";
    }
  }

  async function showEligible(pid: string) {
    try {
      eligible = { ...eligible, [pid]: await getEligibleBeds(pid) };
    } catch (e) {
      error = e instanceof Error ? e.message : "eligible lookup failed";
    }
  }
</script>

<h1>Bed requests</h1>
{#if error}<p class="error">{error}</p>{/if}

<div class="panel">
  <h2>New request</h2>
  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      act(() =>
        createBedRequest({
          person_ref: personRef,
          origin,
          priority,
          ...(targetWard ? { target_ward_pid: targetWard } : {}),
          requirements: {
            isolation: needsIsolation,
            ...(sex ? { sex } : {}),
          },
        }),
      );
    }}
  >
    <input
      placeholder="person:<uuid>"
      bind:value={personRef}
      size="42"
      required
    />
    <select bind:value={origin}>
      <option>ed</option>
      <option>elective</option>
      <option>ward_transfer</option>
      <option>external</option>
      <option>virtual_step_up</option>
    </select>
    <select bind:value={priority}>
      <option>emergency</option>
      <option>urgent</option>
      <option>routine</option>
    </select>
    <select bind:value={targetWard}>
      <option value="">any ward</option>
      {#each data.wards as ward (ward.pid)}
        <option value={ward.pid}>{ward.code}</option>
      {/each}
    </select>
    <select bind:value={sex}>
      <option value="">sex n/a</option>
      <option>female</option>
      <option>male</option>
    </select>
    <label>
      <input type="checkbox" bind:checked={needsIsolation} /> isolation
    </label>
    <button type="submit" class="primary">Queue</button>
  </form>
</div>

<div class="panel">
  <table>
    <thead>
      <tr>
        <th>Priority</th>
        <th>Patient</th>
        <th>Origin</th>
        <th>Requested</th>
        <th>Eligible</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each data.requests as request (request.pid)}
        <tr>
          <td>
            <span
              class="chip {request.priority === 'emergency'
                ? 'danger'
                : request.priority === 'urgent'
                  ? 'warn'
                  : ''}">{request.priority}</span
            >
          </td>
          <td class="muted">{request.person_ref}</td>
          <td>{request.origin}</td>
          <td>{request.requested_at}</td>
          <td>
            {request.eligible_beds ?? "—"}
            {#if request.eligible_beds === 0}
              <span class="chip danger">none — escalate</span>
            {/if}
          </td>
          <td>
            <button onclick={() => showEligible(request.pid)}>
              Show beds
            </button>
            <button onclick={() => act(() => cancelBedRequest(request.pid))}>
              Cancel
            </button>
          </td>
        </tr>
        {#if eligible[request.pid]}
          <tr>
            <td colspan="6">
              <div class="chips">
                {#each eligible[request.pid] ?? [] as bed (bed.bed_pid)}
                  <button
                    onclick={() => act(() => allocateBed(request.pid, bed.bed_pid))}
                  >
                    {bed.ward_code} · {bed.number}
                    {bed.side_room ? "(side room)" : ""}
                    {bed.right_ward ? "" : "(outlier)"}
                  </button>
                {/each}
                {#if (eligible[request.pid] ?? []).length === 0}
                  <span class="muted">no eligible beds</span>
                {/if}
              </div>
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
</div>
