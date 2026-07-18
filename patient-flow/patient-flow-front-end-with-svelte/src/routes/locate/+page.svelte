<script lang="ts">
  // Patient locate: "where is patient X right now?" — an audited,
  // sensitive read (spec `whiteboard.md`).
  import { locatePerson } from "$lib/api/flow";
  import type { Locate } from "$lib/api/types";

  let query = $state("");
  let result = $state<Locate | null>(null);
  let error = $state<string | null>(null);

  async function search(e: SubmitEvent) {
    e.preventDefault();
    error = null;
    result = null;
    try {
      result = await locatePerson(query.trim());
    } catch (err) {
      error =
        err instanceof Error && err.message.includes("404")
          ? "no stay found for that person"
          : err instanceof Error
            ? err.message
            : "lookup failed";
    }
  }
</script>

<h1>Locate a patient</h1>

<div class="panel">
  <form class="row" onsubmit={search}>
    <input
      placeholder="person:<uuid>"
      bind:value={query}
      size="48"
      required
    />
    <button type="submit" class="primary">Locate</button>
  </form>
  <p class="muted">Every locate is recorded in the audit trail.</p>
</div>

{#if error}<p class="error">{error}</p>{/if}

{#if result}
  <div class="panel" data-testid="locate-result">
    <h2>{result.display_name}</h2>
    <div class="chips">
      <span class="chip">{result.status}</span>
      {#if result.site}<span class="chip">{result.site}</span>{/if}
      {#if result.ward}
        <span class="chip ok">{result.ward.code} — {result.ward.name}</span>
      {/if}
      {#if result.bay}<span class="chip">{result.bay}</span>{/if}
      {#if result.bed}<span class="chip">bed {result.bed}</span>{/if}
      {#if result.home_location_note}
        <span class="chip">{result.home_location_note}</span>
      {/if}
    </div>
    <p><a href={`/stays/${result.stay_pid}`}>Open stay</a></p>
  </div>
{/if}
