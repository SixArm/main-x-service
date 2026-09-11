<!--
  Stalled journeys (aging WIP, spec T-14j/T-14l): open instances whose
  last recorded activity is older than the requested idle window,
  most idle first, each row naming which source (a segment, a step, an
  event) that activity actually was. Retroactive: idle-since is the
  activity time itself, never when the silence happened to be noticed.
  Never grouped by actor.
-->
<script lang="ts">
  import type { Stalled } from "$lib/api/tba";

  interface Props {
    stalled: Stalled;
  }
  let { stalled }: Props = $props();

  function lastActivityDate(ms: number): string {
    return new Date(ms).toISOString().slice(0, 10);
  }
</script>

<div data-testid="stalled-list">
  {#if stalled.stalled.length === 0}
    <p class="muted">
      No open instance has been idle past {stalled.idle_days} days.
    </p>
  {:else}
    <table>
      <thead>
        <tr>
          <th scope="col">Subject</th>
          <th scope="col">Urgency</th>
          <th scope="col">Last activity</th>
          <th scope="col">Source</th>
          <th scope="col">Idle days</th>
        </tr>
      </thead>
      <tbody>
        {#each stalled.stalled as row (row.pid)}
          <tr>
            <td>{row.subject_ref}</td>
            <td>{row.urgency}</td>
            <td>{lastActivityDate(row.last_activity_at_ms)}</td>
            <td><code>{row.last_activity_source}</code></td>
            <td>{row.idle_days}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9rem;
  }
  th,
  td {
    text-align: left;
    padding: 0.3rem 0.5rem;
  }
  td {
    font-variant-numeric: tabular-nums;
  }
  .muted {
    opacity: 0.75;
    font-size: 0.9rem;
  }
</style>
