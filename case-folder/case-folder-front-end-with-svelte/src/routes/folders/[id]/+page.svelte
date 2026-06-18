<script lang="ts">
    // Folder detail (`/folders/[id]`) — one folder's record and timeline.
    //
    // Render-only: shows the folder summary (patient, cabinet, volume,
    // status, last moved, notes), a "Move this folder" shortcut, and the
    // full move history. Data comes from the load function's page data.

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import SummaryList from '$lib/components/SummaryList/SummaryList.svelte';
    import SummaryListItem from '$lib/components/SummaryListItem/SummaryListItem.svelte';
    import Separator from '$lib/components/Separator/Separator.svelte';
    import UnitedKingdomNationalHealthServiceNumberView from '$lib/components/UnitedKingdomNationalHealthServiceNumberView/UnitedKingdomNationalHealthServiceNumberView.svelte';
    import { t, statusLabel } from '$lib/i18n.svelte';

    let { data } = $props();
    const folder = $derived(data.folder);
    const history = $derived(data.history);

    // Folder status → Badge colour (green = located, amber = in transit).
    function badgeType(status: string): 'success' | 'warning' | 'info' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    // The patient route is keyed by the bare (spaceless) NHS Number.
    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/folders">{t('folderDetail.backToFolders')}</BackLink>

<h2>
    {folder.title}
    <Badge type={badgeType(folder.status)}>{statusLabel(folder.status)}</Badge>
</h2>
<p>
    <strong>{t('folderDetail.patientPrefix')}</strong>
    <a href="/patients/{nhsSlug(folder.nhsNumber)}">{folder.patientName}</a>
    ·
    <UnitedKingdomNationalHealthServiceNumberView
        class="nhs-number"
        label={t('common.nhsNumber')}
        value={folder.nhsNumber}
    />
</p>

<div class="panel">
    <SummaryList label={t('folderDetail.detailsLabel')}>
        <SummaryListItem term={t('folderDetail.folderTitle')}>{folder.title}</SummaryListItem>
        <SummaryListItem term={t('common.patient')}>
            <a href="/patients/{nhsSlug(folder.nhsNumber)}">{folder.patientName}</a>
        </SummaryListItem>
        <SummaryListItem term={t('folderDetail.currentCabinet')}>{folder.cabinetLabel}</SummaryListItem>
        {#if folder.volumeId}
            <SummaryListItem term={t('common.volume')}>
                <a href="/volumes/{folder.volumeId}">{folder.volumeTitle ?? t('common.volume')}</a>
            </SummaryListItem>
        {/if}
        <SummaryListItem term={t('common.status')}>
            <Badge type={badgeType(folder.status)}>{statusLabel(folder.status)}</Badge>
        </SummaryListItem>
        <SummaryListItem term={t('common.lastMoved')}>
            {folder.lastMovedAt ? new Date(folder.lastMovedAt).toLocaleString('en-GB') : '—'}
        </SummaryListItem>
        {#if folder.notes}
            <SummaryListItem term={t('common.notes')}>{folder.notes}</SummaryListItem>
        {/if}
    </SummaryList>
    <div style="margin-top: var(--nhs-space-3); display:flex; gap: var(--nhs-space-2);">
        <a href="/move?folder={folder.id}" class="button">{t('folderDetail.moveThisFolder')}</a>
    </div>
</div>

<Separator />

<h3>{t('folderDetail.moveHistory')}</h3>
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
        <p>{t('common.noMovesYet')}</p>
    {/if}
</div>
