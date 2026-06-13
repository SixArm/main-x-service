<script lang="ts">
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import SummaryList from '$lib/components/SummaryList/SummaryList.svelte';
    import SummaryListItem from '$lib/components/SummaryListItem/SummaryListItem.svelte';
    import Separator from '$lib/components/Separator/Separator.svelte';
    import UnitedKingdomNationalHealthServiceNumberView from '$lib/components/UnitedKingdomNationalHealthServiceNumberView/UnitedKingdomNationalHealthServiceNumberView.svelte';

    let { data } = $props();
    const folder = $derived(data.folder);
    const history = $derived(data.history);

    function badgeType(status: string): 'success' | 'warning' | 'info' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/folders">Back to folders</BackLink>

<h2>
    {folder.title}
    <Badge type={badgeType(folder.status)}>{folder.status}</Badge>
</h2>
<p>
    <strong>Patient:</strong>
    <a href="/patients/{nhsSlug(folder.nhsNumber)}">{folder.patientName}</a>
    ·
    <UnitedKingdomNationalHealthServiceNumberView
        class="nhs-number"
        label="NHS Number"
        value={folder.nhsNumber}
    />
</p>

<div class="panel">
    <SummaryList label="Folder details">
        <SummaryListItem term="Folder title">{folder.title}</SummaryListItem>
        <SummaryListItem term="Patient">
            <a href="/patients/{nhsSlug(folder.nhsNumber)}">{folder.patientName}</a>
        </SummaryListItem>
        <SummaryListItem term="Current cabinet">{folder.cabinetLabel}</SummaryListItem>
        {#if folder.volumeId}
            <SummaryListItem term="Volume">
                <a href="/volumes/{folder.volumeId}">{folder.volumeTitle ?? 'Volume'}</a>
            </SummaryListItem>
        {/if}
        <SummaryListItem term="Status">
            <Badge type={badgeType(folder.status)}>{folder.status}</Badge>
        </SummaryListItem>
        <SummaryListItem term="Last moved">
            {folder.lastMovedAt ? new Date(folder.lastMovedAt).toLocaleString('en-GB') : '—'}
        </SummaryListItem>
        {#if folder.notes}
            <SummaryListItem term="Notes">{folder.notes}</SummaryListItem>
        {/if}
    </SummaryList>
    <div style="margin-top: var(--nhs-space-3); display:flex; gap: var(--nhs-space-2);">
        <a href="/move?folder={folder.id}" class="button">Move this folder</a>
    </div>
</div>

<Separator />

<h3>Move history</h3>
<div class="move-stack">
    {#each history as move (move.id)}
        <article class="move-card">
            <div class="move-route">
                <span>{move.fromCabinetLabel}</span>
                <span class="move-arrow" aria-hidden="true">→</span>
                <span>{move.toCabinetLabel}</span>
            </div>
            <p class="move-meta">
                {move.movedBy}{#if move.workerRole} ({move.workerRole}){/if}
                · {new Date(move.movedAt).toLocaleString('en-GB')}
                {#if move.reason}· {move.reason}{/if}
            </p>
        </article>
    {/each}
    {#if history.length === 0}
        <p>No moves recorded yet.</p>
    {/if}
</div>
