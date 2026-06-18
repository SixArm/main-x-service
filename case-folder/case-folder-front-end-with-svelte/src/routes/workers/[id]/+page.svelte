<script lang="ts">
    // Worker detail (`/workers/[id]`) — one worker's folder activity.
    //
    // Shows two folder tables (folders this worker moved, and the wider
    // set of all their patients' folders) plus the worker's move log. The
    // `folderTable` snippet is reused for both tables to avoid duplication.

    import type { Folder } from '$lib/store/types';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Separator from '$lib/components/Separator/Separator.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';
    import { t, tf, statusLabel } from '$lib/i18n.svelte';

    let { data } = $props();

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

<!-- Reusable folder table, rendered for both the moved-folders and
     patients'-folders sections below. -->
{#snippet folderTable(folders: Folder[], label: string)}
    <DataTable {label}>
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('common.title')}</th>
                <th scope="col">{t('common.patient')}</th>
                <th scope="col">{t('common.cabinet')}</th>
                <th scope="col">{t('common.status')}</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each folders as folder (folder.id)}
                <DataTableRow>
                    <DataTableTD><a href="/folders/{folder.id}">{folder.title}</a></DataTableTD>
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(folder.nhsNumber)}">{folder.patientName}</a>
                    </DataTableTD>
                    <DataTableTD>{folder.cabinetLabel}</DataTableTD>
                    <DataTableTD>
                        <Badge type={badgeType(folder.status)}>{statusLabel(folder.status)}</Badge>
                    </DataTableTD>
                </DataTableRow>
            {/each}
        </DataTableBody>
    </DataTable>
{/snippet}

<BackLink href="/workers">{t('workerDetail.backToWorkers')}</BackLink>

<h2>{data.worker.name}</h2>
{#if data.worker.role}<p><em>{data.worker.role}</em></p>{/if}

<div class="panel">
    <h3>{tf('workerDetail.foldersMoved', { n: data.movedFolders.length })}</h3>
    {#if data.movedFolders.length > 0}
        {@render folderTable(data.movedFolders, t('workerDetail.foldersMovedTable'))}
    {:else}
        <p>{t('workerDetail.noMovedFolders')}</p>
    {/if}
</div>

<div class="panel">
    <h3>{tf('workerDetail.patientsFolders', { n: data.patientFolders.length })}</h3>
    <p>{t('workerDetail.patientsFoldersIntro')}</p>
    {#if data.patientFolders.length > 0}
        {@render folderTable(data.patientFolders, t('workerDetail.patientsFoldersTable'))}
    {:else}
        <p>{t('workerDetail.noPatientFolders')}</p>
    {/if}
</div>

<Separator />

<h3>{t('workerDetail.movesByWorker')}</h3>
<div class="move-stack">
    {#each data.moves as move (move.id)}
        <article class="move-card">
            <div class="move-route">
                <a href="/history/{move.id}"><strong>{move.folderTitle}</strong></a>:
                <span>{move.fromCabinetLabel}</span>
                <span class="move-arrow" aria-hidden="true">→</span>
                <span>{move.toCabinetLabel}</span>
            </div>
            <p class="move-meta">
                {move.patientName} · {new Date(move.movedAt).toLocaleString('en-GB')}
                {#if move.reason}· {move.reason}{/if}
            </p>
        </article>
    {/each}
    {#if data.moves.length === 0}
        <p>{t('common.noMovesYet')}</p>
    {/if}
</div>
