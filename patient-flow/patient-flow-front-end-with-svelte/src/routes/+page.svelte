<script lang="ts">
  let { data } = $props();
  let wards = $derived(data.glance.wards);
</script>

<h1>Wards</h1>

<div class="panel">
  <table>
    <thead>
      <tr>
        <th>Code</th>
        <th>Ward</th>
        <th>Kind</th>
        <th>Beds</th>
        <th>Occupied</th>
        <th>Available</th>
        <th>Ready</th>
        <th>DTOC</th>
        <th>Board</th>
      </tr>
    </thead>
    <tbody>
      {#each wards as ward (ward.ward_pid)}
        <tr>
          <td><strong>{ward.code}</strong></td>
          <td>{ward.name}</td>
          <td>
            {ward.kind}
            {#if ward.escalation}<span class="chip warn">esc</span>{/if}
            {#if ward.closed_to_admissions}
              <span class="chip danger">closed</span>
            {/if}
          </td>
          <td>{ward.beds_total}</td>
          <td>{ward.occupied}</td>
          <td>{ward.available}</td>
          <td>{ward.discharge_ready}</td>
          <td>{ward.dtoc}</td>
          <td>
            <a href={`/wards/${ward.ward_pid}/whiteboard`}>whiteboard</a>
            ·
            <a href={`/wards/${ward.ward_pid}/kiosk`}>kiosk</a>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
<p class="muted">as of {data.glance.as_of}</p>
