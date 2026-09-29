<!--
  Dashboard — the signed-in home: the ward list with live counts.

  Props:
    - glance: AtAGlance — the at-a-glance payload loaded by the home route.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte.js";
  import type { AtAGlance } from "$lib/api/types";

  let { glance }: { glance: AtAGlance } = $props();
  let wards = $derived(glance.wards);
</script>

<h1>{t("nav.wards")}</h1>

<div class="panel">
  <table>
    <thead>
      <tr>
        <th>{t("home.col.code")}</th>
        <th>{t("home.col.ward")}</th>
        <th>{t("home.col.kind")}</th>
        <th>{t("home.col.beds")}</th>
        <th>{t("home.col.occupied")}</th>
        <th>{t("home.col.available")}</th>
        <th>{t("home.col.ready")}</th>
        <th>{t("home.col.dtoc")}</th>
        <th>{t("home.col.board")}</th>
      </tr>
    </thead>
    <tbody>
      {#each wards as ward (ward.ward_pid)}
        <tr>
          <td><strong>{ward.code}</strong></td>
          <td>{ward.name}</td>
          <td>
            {ward.kind}
            {#if ward.escalation}<span class="chip warn">{t("home.chip.esc")}</span>{/if}
            {#if ward.closed_to_admissions}
              <span class="chip danger">{t("home.chip.closed")}</span>
            {/if}
          </td>
          <td>{ward.beds_total}</td>
          <td>{ward.occupied}</td>
          <td>{ward.available}</td>
          <td>{ward.discharge_ready}</td>
          <td>{ward.dtoc}</td>
          <td>
            <a href={`/wards/${ward.ward_pid}/whiteboard`}>{t("home.link.whiteboard")}</a>
            ·
            <a href={`/wards/${ward.ward_pid}/kiosk`}>{t("home.link.kiosk")}</a>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
<p class="muted">{t("home.as_of")} {glance.as_of}</p>
