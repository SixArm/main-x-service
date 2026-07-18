<script lang="ts">
  // One bed card (spec `whiteboard.md`): the state colour is the left
  // border; occupied beds show the patient row and journey chips;
  // empty beds show their cycle position and (optionally) the cleaning
  // actions. Pure presentational — actions are callbacks.
  import type { BedCard } from "$lib/api/types";

  let {
    card,
    masked = false,
    onopen,
    oncleanstart,
    oncleancomplete,
  }: {
    card: BedCard;
    /** Corridor mode: suppress patient-identifying fields client-side
     *  too (the server already redacts under the ABAC mask). */
    masked?: boolean;
    onopen?: (stayPid: string) => void;
    oncleanstart?: (bedPid: string) => void;
    oncleancomplete?: (bedPid: string, deep: boolean) => void;
  } = $props();

  const stateLabel: Record<string, string> = {
    available: "Available",
    reserved: "Reserved",
    occupied: "Occupied",
    awaiting_clean: "Awaiting clean",
    cleaning: "Cleaning",
    closed: "Closed",
  };

  let name = $derived(
    masked ? "•••" : (card.display_name ?? ""),
  );
  let alerts = $derived(masked ? [] : card.alerts);
</script>

<div class="bed-card state-{card.state}" data-bed={card.number}>
  <div class="head">
    <span class="number">{card.number}</span>
    <span class="state-label">
      {stateLabel[card.state] ?? card.state}
      {#if card.closure_reason}({card.closure_reason}){/if}
    </span>
  </div>

  {#if card.stay_pid}
    {#if onopen}
      <button
        class="patient"
        style="all: unset; cursor: pointer; font-weight: 600; font-size: 1.02rem;"
        onclick={() => onopen?.(card.stay_pid ?? "")}>{name}</button
      >
    {:else}
      <div class="patient">{name}</div>
    {/if}
    {#if !masked && (card.named_nurse_ref || card.consultant_ref)}
      <div class="staff">
        {card.named_nurse_ref ?? ""}
        {card.consultant_ref ?? ""}
      </div>
    {/if}
    <div class="chips">
      {#if card.edd_missing}
        <span class="chip warn">EDD missing</span>
      {:else if card.edd_overdue}
        <span class="chip danger">EDD {card.edd} overdue</span>
      {:else if card.edd}
        <span class="chip">EDD {card.edd}</span>
      {/if}
      {#if card.ccd_met}<span class="chip ok">CCD met</span>{/if}
      {#if card.discharge_pathway}
        <span class="chip">{card.discharge_pathway.toUpperCase()}</span>
      {/if}
      {#if card.dtoc}
        <span class="chip danger">DTOC</span>
      {:else if card.discharge_ready}
        <span class="chip ok">Ready</span>
      {/if}
      {#if card.senior_review_today}<span class="chip ok">Reviewed</span>{/if}
      {#if card.red_green_today === "red"}
        <span class="chip red-day">Red</span>
      {:else if card.red_green_today === "green"}
        <span class="chip green-day">Green</span>
      {/if}
      {#each card.infection as flag (flag.precaution + (flag.organism ?? ""))}
        <span class="chip infection {flag.status}">
          {flag.organism ?? flag.precaution}
          {flag.status === "suspected" ? "?" : ""}
        </span>
      {/each}
      {#each alerts as alert (alert)}
        <span class="chip warn">{alert}</span>
      {/each}
    </div>
  {:else}
    <div class="chips">
      {#if card.side_room}<span class="chip">Side room</span>{/if}
      {#if card.deep_clean_required}
        <span class="chip danger">Deep clean required</span>
      {/if}
      {#if card.state === "awaiting_clean" && oncleanstart}
        <button onclick={() => oncleanstart?.(card.bed_pid)}>
          Start clean
        </button>
      {/if}
      {#if card.state === "cleaning" && oncleancomplete}
        <button
          onclick={() =>
            oncleancomplete?.(card.bed_pid, card.deep_clean_required)}
        >
          {card.deep_clean_required ? "Deep clean done" : "Clean done"}
        </button>
      {/if}
    </div>
  {/if}
</div>
