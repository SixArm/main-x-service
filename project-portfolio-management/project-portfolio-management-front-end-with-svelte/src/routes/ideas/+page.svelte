<!--
  Idea board (`/ideas`, PPM-2): capture, vote (most-voted first),
  dismiss, and convert into a draft proposal — the funnel's first
  stage (idea → proposal → work item).
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import { PpmClient, type Idea } from "$lib/api/ppm";
  import { COLLECTIONS, type Collection } from "$lib/api/types";

  const ppm = PpmClient.withFetch();
  let ideas = $state<Idea[]>([]);
  let error = $state<string | null>(null);
  let title = $state("");
  let pitch = $state("");
  let convertTarget = $state<Collection>("projects");

  async function refresh() {
    try {
      ideas = await ppm.listIdeas();
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  }
  onMount(refresh);

  async function act(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
      await refresh();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.actionFailed");
    }
  }

  async function capture(event: SubmitEvent) {
    event.preventDefault();
    await act(() => ppm.createIdea({ title, pitch: pitch || undefined }));
    title = "";
    pitch = "";
  }
</script>

<svelte:head><title>Ideas — PPM</title></svelte:head>

<h1>{t("ppm.ideas.title")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

<form class="row" onsubmit={capture}>
  <input placeholder={t("ppm.ideas.formTitle")} bind:value={title} required />
  <input placeholder={t("ppm.ideas.formPitch")} bind:value={pitch} size="40" />
  <button class="button primary" type="submit">{t("ppm.ideas.capture")}</button>
</form>

<p class="row small">
  {t("ppm.ideas.convertTarget")}
  <select bind:value={convertTarget} aria-label="Convert target">
    {#each COLLECTIONS as target (target)}<option value={target}>{target}</option>{/each}
  </select>
</p>

<ul class="ideas">
  {#each ideas as idea (idea.pid)}
    <li>
      <button class="button small" onclick={() => act(() => ppm.voteIdea(idea.pid))}>
        ▲ {idea.votes}
      </button>
      <span class="body">
        <strong>{idea.title}</strong>
        {#if idea.pitch}<span class="small muted"> — {idea.pitch}</span>{/if}
      </span>
      <button
        class="button primary small"
        onclick={() => act(() => ppm.convertIdea(idea.pid, convertTarget))}
      >
        {t("ppm.ideas.toProposal")}
      </button>
      <button class="button danger small" onclick={() => act(() => ppm.dismissIdea(idea.pid))}>
        {t("ppm.ideas.dismiss")}
      </button>
    </li>
  {/each}
</ul>
{#if ideas.length === 0 && !error}<p class="small muted">{t("ppm.ideas.empty")}</p>{/if}

<style>
  .row { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; margin: 0.8rem 0; }
  .ideas { list-style: none; padding: 0; }
  .ideas li {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    padding: 0.4rem 0;
    border-bottom: 1px solid var(--border, #ddd);
  }
  .ideas .body { flex: 1; }
</style>
