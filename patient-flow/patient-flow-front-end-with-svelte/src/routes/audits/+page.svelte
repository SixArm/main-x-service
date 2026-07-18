<script lang="ts">
  // The audit trail view; the ward+since filter is the clinical
  // handover query (spec `audit.md`).
  import { goto } from "$app/navigation";

  let { data } = $props();
  // svelte-ignore state_referenced_locally — URL params deliberately
  // seed the form once; the user owns the fields after that.
  let ward = $state(data.ward ?? "");
  // svelte-ignore state_referenced_locally
  let since = $state(
    data.since ?? new Date(Date.now() - 12 * 3600_000).toISOString(),
  );
</script>

<h1>Audit trail</h1>

<div class="panel">
  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      goto(
        ward
          ? `/audits?ward=${ward}&since=${encodeURIComponent(since)}`
          : "/audits",
      );
    }}
  >
    <select bind:value={ward}>
      <option value="">all (recent)</option>
      {#each data.wards as w (w.pid)}
        <option value={w.pid}>{w.code} — handover</option>
      {/each}
    </select>
    <input size="28" bind:value={since} />
    <button type="submit">Show</button>
  </form>
</div>

<div class="panel">
  <table>
    <thead>
      <tr>
        <th>When</th>
        <th>Entity</th>
        <th>Action</th>
        <th>Actor</th>
        <th>Detail</th>
      </tr>
    </thead>
    <tbody>
      {#each data.entries as entry (entry.id)}
        <tr>
          <td>{entry.created_at}</td>
          <td>{entry.entity}</td>
          <td>{entry.action}</td>
          <td>{entry.actor ?? "—"}</td>
          <td class="muted">
            {entry.snapshot ? JSON.stringify(entry.snapshot) : ""}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
