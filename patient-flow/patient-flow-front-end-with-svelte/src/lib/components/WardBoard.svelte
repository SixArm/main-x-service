<script lang="ts">
  // The ward whiteboard: bay-ordered bed cards, polled every
  // BOARD_POLL_MS with a visible `as_of` (spec `whiteboard.md` —
  // a wall screen is honest about staleness).
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { BOARD_POLL_MS } from "$lib/config";
  import { bedTransition, getWhiteboard } from "$lib/api/flow";
  import type { Whiteboard } from "$lib/api/types";
  import BedCard from "./BedCard.svelte";

  let {
    wardPid,
    initial,
    masked = false,
  }: { wardPid: string; initial: Whiteboard; masked?: boolean } = $props();

  // svelte-ignore state_referenced_locally — the server-loaded board
  // deliberately seeds the poll state once; refresh() owns it after.
  let board = $state(initial);
  let error = $state<string | null>(null);

  async function refresh() {
    try {
      board = await getWhiteboard(wardPid);
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : "refresh failed";
    }
  }

  onMount(() => {
    const timer = setInterval(refresh, BOARD_POLL_MS);
    return () => clearInterval(timer);
  });

  async function act(action: () => Promise<unknown>) {
    try {
      await action();
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : "action failed";
    }
  }

  let asOf = $derived(new Date(board.as_of).toLocaleTimeString());
</script>

<div class="board-meta">
  <strong>{board.ward_code}</strong>
  <span>{board.ward_name}</span>
  {#if board.kind === "virtual"}<span class="chip">Virtual ward</span>{/if}
  {#if board.escalation}<span class="chip warn">Escalation</span>{/if}
  {#if board.closed_to_admissions}
    <span class="chip danger">Closed to admissions</span>
  {/if}
  {#if board.masked || masked}<span class="chip">Masked</span>{/if}
  <span data-testid="as-of">as of {asOf}</span>
  {#if error}<span class="error">{error}</span>{/if}
</div>

<div class="board">
  {#each board.cards as card (card.bed_pid)}
    <BedCard
      {card}
      masked={masked || board.masked}
      onopen={(stayPid) => goto(`/stays/${stayPid}`)}
      oncleanstart={(bedPid) =>
        act(() => bedTransition(bedPid, "clean_start"))}
      oncleancomplete={(bedPid, deep) =>
        act(() =>
          bedTransition(bedPid, "clean_complete", { deep_clean_done: deep }),
        )}
    />
  {/each}
</div>
