<!--
  Case detail route (`/[pid]`).

  Purpose: load one case, render its fields, and offer edit / delete /
  check-duplicates actions.

  State:
    - record     : the loaded Case (null until fetched / on error).
    - loading,error: load-phase UI flags.
    - duplicates : null until a check runs, then the scored hits (minus self).
    - checking   : disables the button while a check is in flight.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { CaseRepository } from "$lib/api/cases";
  import LinksPanel from "$lib/components/LinksPanel.svelte";
  import type { Case, ScoredRef } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  const repo = CaseRepository.withFetch();
  // Route param; `?? ""` satisfies strict typing (params may be undefined).
  const pid = page.params.pid ?? "";

  let record = $state<Case | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let duplicates = $state<ScoredRef[] | null>(null);
  let checking = $state(false);

  // Fetch the record on mount.
  onMount(async () => {
    try {
      record = await repo.get(pid);
    } catch (err) {
      error = err instanceof Error ? err.message : t("detail.notFound");
    } finally {
      loading = false;
    }
  });

  // Soft-delete this case, then return to the list.
  async function handleDelete() {
    await repo.remove(pid);
    await goto("/");
  }

  // Score this record against the corpus, excluding the record itself from
  // the results so it never reports itself as its own duplicate.
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

<svelte:head><title>{record?.title ?? "Case"} — Main X</title></svelte:head>

{#if loading}
  <p>{t("detail.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if record}
  <h1>{record.title}</h1>
  <div class="surface stack">
    {#if record.case_type}
      <div>
        <strong>{t("detail.caseType")}</strong>
        {typeof record.case_type === "string"
          ? record.case_type
          : record.case_type.Custom}
      </div>
    {/if}
    {#if record.status}
      <div>
        <strong>{t("detail.status")}</strong>
        {typeof record.status === "string"
          ? record.status
          : record.status.Custom}
      </div>
    {/if}
    {#if record.priority}<div>
        <strong>{t("detail.priority")}</strong>
        {record.priority}
      </div>{/if}
    {#if record.agency_name}<div>
        <strong>{t("detail.agency")}</strong>
        {record.agency_name}
      </div>{/if}
    {#if record.case_number}
      <div><strong>{t("detail.caseNumber")}</strong> <code>{record.case_number}</code></div>
    {/if}
    {#if record.opened_date}<div>
        <strong>{t("detail.opened")}</strong>
        {record.opened_date}
      </div>{/if}
    {#if record.subjects && record.subjects.length > 0}
      <div><strong>{t("detail.subjects")}</strong> {record.subjects.join(", ")}</div>
    {/if}
    {#if record.identifiers && record.identifiers.length > 0}
      <div>
        <strong>{t("detail.identifiers")}</strong>
        <ul>
          {#each record.identifiers as id, i (i)}
            <li>
              {typeof id.scheme === "string"
                ? id.scheme
                : `Custom(${id.scheme.Custom})`}:
              <code>{id.value}</code>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
    {#if record.keywords && record.keywords.length > 0}
      <div><strong>{t("detail.keywords")}</strong> {record.keywords.join(", ")}</div>
    {/if}
    <div><strong>{t("detail.id")}</strong> <code>{pid}</code></div>
  </div>

  <div class="row" style="margin-top:1rem">
    <a class="button" href={`/${pid}/edit`}>{t("detail.edit")}</a>
    <button class="button" onclick={handleCheckDuplicates} disabled={checking}>
      {checking ? t("detail.checking") : t("detail.checkDuplicates")}
    </button>
    <button onclick={handleDelete}>{t("detail.delete")}</button>
  </div>

  <!-- `duplicates` is null before any check; show results only once run. -->
  {#if duplicates}
    <h2>{t("detail.potentialDuplicates")}</h2>
    {#if duplicates.length === 0}
      <p>{t("detail.noneAboveThreshold")}</p>
    {:else}
      <ul class="stack">
        {#each duplicates as dup (dup.pid)}
          <li class="surface row">
            <a href={`/${dup.pid}`}>{dup.title}</a>
            <span>{dup.score.toFixed(3)} · {dup.confidence}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}

  <!--
    Cross-service links: the `subject_of` (case → person) edges this case
    asserts. Deliberately a plain, labelled section at the foot of the
    record rather than an inline control — the edge is high-sensitivity
    (cross-service-linking §10) and asserting one is a considered act.
  -->
  <LinksPanel casePid={pid} />
{/if}
