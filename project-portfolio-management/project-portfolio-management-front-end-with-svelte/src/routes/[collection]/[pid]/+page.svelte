<!--
  Work-item detail route (`/[collection]/[pid]`).

  Loads one work item, renders its fields, and offers edit / delete /
  check-duplicates (within the collection; cross-kind never matches).
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { WorkItemRepository } from "$lib/api/work-items";
  import type { ScoredRef, WorkItem } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();
  const collection = $derived(data.collection);
  const pid = page.params.pid ?? "";
  const repo = $derived(WorkItemRepository.withFetch(collection));

  let record = $state<WorkItem | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let duplicates = $state<ScoredRef[] | null>(null);
  let checking = $state(false);

  onMount(async () => {
    try {
      record = await repo.get(pid);
    } catch (err) {
      error = err instanceof Error ? err.message : t("detail.notFound");
    } finally {
      loading = false;
    }
  });

  async function handleDelete() {
    await repo.remove(pid);
    await goto(`/${collection}`);
  }

  async function handleCheckDuplicates() {
    if (!record) return;
    checking = true;
    try {
      const hits = await repo.checkDuplicates(record);
      duplicates = hits.filter((h) => h.pid !== pid);
    } catch (err) {
      error = err instanceof Error ? err.message : t("detail.checkFailed");
    } finally {
      checking = false;
    }
  }
</script>

<svelte:head><title>{record?.name ?? "Work item"} — Main X</title></svelte:head>

{#if loading}
  <p>{t("detail.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if record}
  <h1>{record.name}</h1>
  <p class="small">
    <a class="button small" href={`/${collection}/${pid}/governance`}>{t("ppm.common.governance")}</a>
    {#if collection === "portfolios"}
      <a class="button small" href={`/${collection}/${pid}/schedule`}>{t("ppm.common.schedule")}</a>
    {/if}
  </p>
  <div class="surface stack">
    <div><strong>{t("detail.caseType")}</strong> {record.kind}</div>
    {#if record.status}
      <div>
        <strong>{t("detail.status")}</strong>
        {typeof record.status === "string" ? record.status : record.status.Custom}
      </div>
    {/if}
    {#if record.code}<div><strong>{t("detail.caseNumber")}</strong> <code>{record.code}</code></div>{/if}
    {#if record.owner_org_name}<div><strong>{t("detail.agency")}</strong> {record.owner_org_name}</div>{/if}
    {#if record.portfolio_ref}<div><strong>{t("detail.opened")}</strong> <code>{record.portfolio_ref}</code></div>{/if}
    {#if record.keywords && record.keywords.length > 0}
      <div><strong>{t("detail.keywords")}</strong> {record.keywords.join(", ")}</div>
    {/if}
    {#if record.tags && record.tags.length > 0}
      <div><strong>{t("detail.subjects")}</strong> {record.tags.join(", ")}</div>
    {/if}
    {#if record.identifiers && record.identifiers.length > 0}
      <div>
        <strong>{t("detail.identifiers")}</strong>
        <ul>
          {#each record.identifiers as id, i (i)}
            <li>
              {typeof id.scheme === "string" ? id.scheme : `Custom(${id.scheme.Custom})`}:
              <code>{id.value}</code>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
    <div><strong>{t("detail.id")}</strong> <code>{pid}</code></div>
  </div>

  <div class="row" style="margin-top:1rem">
    <a class="button" href={`/${collection}/${pid}/edit`}>{t("detail.edit")}</a>
    <button class="button" onclick={handleCheckDuplicates} disabled={checking}>
      {checking ? t("detail.checking") : t("detail.checkDuplicates")}
    </button>
    <button onclick={handleDelete}>{t("detail.delete")}</button>
  </div>

  {#if duplicates}
    <h2>{t("detail.potentialDuplicates")}</h2>
    {#if duplicates.length === 0}
      <p>{t("detail.noneAboveThreshold")}</p>
    {:else}
      <ul class="stack">
        {#each duplicates as dup (dup.pid)}
          <li class="surface row">
            <a href={`/${collection}/${dup.pid}`}>{dup.name}</a>
            <span>{dup.score.toFixed(3)} · {dup.confidence}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
{/if}
