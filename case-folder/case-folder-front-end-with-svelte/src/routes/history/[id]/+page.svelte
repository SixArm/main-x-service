<script lang="ts">
    // Move-event detail (`/history/[id]`) — one audited folder move.
    //
    // Render-only. Shows the move (folder, patient, from/to cabinets,
    // who moved it, reason) and cross-links to the patient's other
    // folders supplied by the load function.

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Separator from '$lib/components/Separator/Separator.svelte';
    import SummaryList from '$lib/components/SummaryList/SummaryList.svelte';
    import SummaryListItem from '$lib/components/SummaryListItem/SummaryListItem.svelte';

    let { data } = $props();
    const move = $derived(data.move);

    // Folder status → Badge colour (green = located, amber = in transit).
    function badgeType(status: string): 'success' | 'warning' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    // The patient route is keyed by the bare (spaceless) NHS Number.
    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/history">Back to move history</BackLink>

<h2>Move event</h2>
<p>{new Date(move.movedAt).toLocaleString('en-GB')}</p>

<div class="panel">
    <h3>Folder involved</h3>
    <SummaryList label="Move event details">
        <SummaryListItem term="Folder">
            <a href="/folders/{move.folderId}">{move.folderTitle}</a>
        </SummaryListItem>
        <SummaryListItem term="Patient">
            <a href="/patients/{nhsSlug(move.nhsNumber)}">{move.patientName}</a>
            (<span class="nhs-number">{move.nhsNumber}</span>)
        </SummaryListItem>
        <SummaryListItem term="From">{move.fromCabinetLabel}</SummaryListItem>
        <SummaryListItem term="To">{move.toCabinetLabel}</SummaryListItem>
        <SummaryListItem term="Moved by">
            {move.movedBy}{#if move.workerRole} ({move.workerRole}){/if}
        </SummaryListItem>
        <SummaryListItem term="Reason">{move.reason ?? '—'}</SummaryListItem>
    </SummaryList>
</div>

<Separator />

<h3>Other folders for {move.patientName}</h3>
{#if data.folders.length > 0}
    <ul class="folder-links">
        {#each data.folders as folder (folder.id)}
            <li>
                <a href="/folders/{folder.id}">{folder.title}</a>
                — <Badge type={badgeType(folder.status)}>{folder.status}</Badge>
                <span class="muted">{folder.cabinetLabel}</span>
            </li>
        {/each}
    </ul>
{:else}
    <p>No other folders for this patient.</p>
{/if}
