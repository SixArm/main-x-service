<!--
  Idea board (`/ideas`, PPM-2): capture, vote (most-voted first),
  dismiss, and convert into a draft proposal — the funnel's first
  stage (idea → proposal → work item).
-->
<script lang="ts">
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
      error = err instanceof Error ? err.message : "load failed";
    }
  }
  onMount(refresh);

  async function act(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
      await refresh();
    } catch (err) {
      error = err instanceof Error ? err.message : "action failed";
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

<h1>Ideas</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

<form class="row" onsubmit={capture}>
  <input placeholder="Idea title" bind:value={title} required />
  <input placeholder="Pitch" bind:value={pitch} size="40" />
  <button class="button primary" type="submit">Capture</button>
</form>

<p class="row small">
  Convert target:
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
        To proposal
      </button>
      <button class="button danger small" onclick={() => act(() => ppm.dismissIdea(idea.pid))}>
        Dismiss
      </button>
    </li>
  {/each}
</ul>
{#if ideas.length === 0 && !error}<p class="small muted">No open ideas — capture one above.</p>{/if}

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
