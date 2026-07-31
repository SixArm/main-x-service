<!--
  The asset library.

  Alt text is the load-bearing column. A CMS that makes it easy to skip
  produces an inaccessible site, so a missing description is shown as a
  finding with its consequence — the page will not publish — rather than
  as a quiet blank (`../../spec/assets.md`).

  Orphans are **reported, never deleted**, and the page says so: an
  operator reading "referenced by nothing" should not have to wonder
  whether the tool already acted on it.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import * as cms from "$lib/api/cms";
  import { bytes } from "$lib/format";
  import SitePicker from "$lib/components/SitePicker.svelte";
  import type { Asset } from "$lib/api/cms";

  let site = $state<string | null>(null);
  let assets = $state<Asset[]>([]);
  let orphans = $state<{ pid: string; byte_size: number }[]>([]);
  let reclaimable = $state(0);
  let quota = $state<{ used_bytes: number; quota_bytes: number } | null>(null);
  let failure = $state<string | null>(null);

  $effect(() => {
    const pid = site;
    if (!pid) return;
    Promise.all([cms.listAssets(pid), cms.orphanAssets(pid), cms.assetQuota(pid)])
      .then(([rows, orphaned, storage]) => {
        assets = rows as Asset[];
        orphans = orphaned.orphans;
        reclaimable = orphaned.bytes_reclaimable;
        quota = storage;
      })
      .catch((e: unknown) => (failure = String(e)));
  });

  const orphaned = $derived(new Set(orphans.map((o) => o.pid)));
  const undescribed = $derived(
    assets.filter((a) => a.kind === "image" && !a.alt_text?.trim()).length,
  );
</script>

<svelte:head><title>{t("nav.assets")}</title></svelte:head>

<h1>{t("nav.assets")}</h1>
<SitePicker bind:site />

{#if failure}
  <div class="panel error">{t("common.error")}: {failure}</div>
{:else}
  <div class="tiles">
    <div class="tile">
      <div class="value">{assets.length}</div>
      <div class="label">{t("nav.assets")}</div>
    </div>
    <div class="tile">
      <div class="value">{undescribed}</div>
      <div class="label">{t("assets.altMissing")}</div>
    </div>
    <div class="tile">
      <div class="value">{orphans.length}</div>
      <div class="label">{t("assets.orphans")}</div>
    </div>
    {#if quota}
      <div class="tile">
        <div class="value">{bytes(quota.used_bytes)}</div>
        <div class="label">{t("assets.storage")}</div>
      </div>
    {/if}
  </div>

  {#if undescribed > 0}
    <p class="ahead">{t("assets.altGate")}</p>
  {/if}
  {#if orphans.length > 0}
    <p class="muted">
      {t("assets.orphansNote")} · {bytes(reclaimable)}
    </p>
  {/if}

  <table>
    <thead>
      <tr>
        <th>{t("common.title")}</th>
        <th>{t("entry.type")}</th>
        <th>{t("common.size")}</th>
        <th>alt</th>
        <th>{t("common.status")}</th>
      </tr>
    </thead>
    <tbody>
      {#each assets as asset (asset.pid)}
        <tr>
          <td>{asset.title ?? asset.original_filename ?? asset.pid}</td>
          <td>{asset.mime}</td>
          <td>{bytes(asset.byte_size)}</td>
          <td>
            {#if asset.kind !== "image"}
              —
            {:else if asset.alt_text?.trim()}
              {asset.alt_text}
            {:else}
              <span class="chip danger">{t("assets.altMissing")}</span>
            {/if}
          </td>
          <td>
            {#if orphaned.has(asset.pid)}
              <span class="chip warn">{t("assets.orphans")}</span>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if assets.length === 0}<p class="no-data">{t("common.noData")}</p>{/if}
{/if}
