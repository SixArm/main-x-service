<script lang="ts">
  let { data } = $props();
  let tiles = $derived(data.glance.site_tiles);
</script>

<h1>Hospital at a glance</h1>

<div class="tiles">
  <div class="tile">
    <div class="value">{tiles.available_now}</div>
    <div class="label">Beds available now</div>
  </div>
  <div class="tile">
    <div class="value">{tiles.predicted_available_by_midnight}</div>
    <div class="label">Predicted by midnight</div>
  </div>
  <div class="tile">
    <div class="value">{tiles.dtoc}</div>
    <div class="label">DTOC</div>
  </div>
  <div class="tile">
    <div class="value">
      {tiles.open_requests.emergency + tiles.open_requests.urgent +
        tiles.open_requests.routine}
    </div>
    <div class="label">
      Open requests (E {tiles.open_requests.emergency} · U
      {tiles.open_requests.urgent} · R {tiles.open_requests.routine})
    </div>
  </div>
  <div class="tile">
    <div class="value">{tiles.virtual_ward_census}</div>
    <div class="label">Virtual ward census</div>
  </div>
  <div class="tile">
    <div class="value">{tiles.escalation_beds_open}</div>
    <div class="label">Escalation beds open</div>
  </div>
</div>

<div class="panel">
  <table>
    <thead>
      <tr>
        <th>Ward</th>
        <th>Occ %</th>
        <th>Occ</th>
        <th>Avail</th>
        <th>Resv</th>
        <th>Clean</th>
        <th>Closed</th>
        <th>EDD today</th>
        <th>EDD overdue</th>
        <th>Ready</th>
        <th>DTOC</th>
        <th>LOS&gt;7</th>
        <th>LOS&gt;21</th>
      </tr>
    </thead>
    <tbody>
      {#each data.glance.wards as ward (ward.ward_pid)}
        <tr>
          <td>
            <a href={`/wards/${ward.ward_pid}/whiteboard`}>
              <strong>{ward.code}</strong>
            </a>
            {#if ward.closed_to_admissions}
              <span class="chip danger">closed</span>
            {/if}
          </td>
          <td>{ward.occupancy_pct}%</td>
          <td>{ward.occupied}</td>
          <td>{ward.available}</td>
          <td>{ward.reserved}</td>
          <td>{ward.awaiting_clean + ward.cleaning}</td>
          <td>
            {ward.closed}{#if ward.closed_for_infection > 0}&nbsp;({ward.closed_for_infection}
              inf){/if}
          </td>
          <td>{ward.expected_discharges_today}</td>
          <td>{ward.edd_overdue}</td>
          <td>{ward.discharge_ready}</td>
          <td>{ward.dtoc}</td>
          <td>{ward.long_stay_7}</td>
          <td>{ward.long_stay_21}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
<p class="muted">as of {data.glance.as_of}</p>
