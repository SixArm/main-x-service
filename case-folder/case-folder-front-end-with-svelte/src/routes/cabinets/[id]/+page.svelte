<script lang="ts">
    // Cabinet detail (`/cabinets/[id]`) — current contents + history.
    //
    // Render-only. Shows the folders currently in the cabinet and the
    // full in/out presence timeline (derived from the move audit log;
    // a null `leftAt` means the folder is still present).

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

    // Format an ISO timestamp in UK locale for the timeline columns.
    function when(iso: string): string {
        return new Date(iso).toLocaleString('en-GB');
    }

    // The patient route is keyed by the bare (spaceless) NHS Number.
    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/cabinets">{t('cabinetDetail.backToCabinets')}</BackLink>

<h2>{data.place.name}</h2>
{#if data.place.container_path}<p>{data.place.container_path}</p>{/if}

<div class="panel">
    <h3>{tf('cabinetDetail.currentFolders', { n: data.folders.length })}</h3>
    {#if data.folders.length > 0}
        <DataTable label={t('cabinetDetail.currentFoldersTable')}>
            <DataTableHead>
                <DataTableRow>
                    <th scope="col">{t('common.title')}</th>
                    <th scope="col">{t('common.patient')}</th>
                    <th scope="col">{t('common.status')}</th>
                </DataTableRow>
            </DataTableHead>
            <DataTableBody>
                {#each data.folders as folder (folder.id)}
                    <DataTableRow>
                        <DataTableTD><a href="/folders/{folder.id}">{folder.title}</a></DataTableTD>
                        <DataTableTD>
                            <a href="/patients/{nhsSlug(folder.nhsNumber)}">{folder.patientName}</a>
                        </DataTableTD>
                        <DataTableTD>
                            <Badge type={badgeType(folder.status)}>{statusLabel(folder.status)}</Badge>
                        </DataTableTD>
                    </DataTableRow>
                {/each}
            </DataTableBody>
        </DataTable>
    {:else}
        <p>{t('cabinetDetail.empty')}</p>
    {/if}
</div>

<Separator />

<h3>{t('cabinetDetail.presenceHistory')}</h3>
<p>{t('cabinetDetail.presenceIntro')}</p>
<div class="panel">
    <DataTable label={t('cabinetDetail.presenceTable')} caption={t('cabinetDetail.presenceCaption')}>
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('common.folder')}</th>
                <th scope="col">{t('common.patient')}</th>
                <th scope="col">{t('common.entered')}</th>
                <th scope="col">{t('common.left')}</th>
                <th scope="col">{t('cabinetDetail.colReasonLeft')}</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each data.presences as p (p.folderId + p.enteredAt)}
                <DataTableRow>
                    <DataTableTD><a href="/folders/{p.folderId}">{p.folderTitle}</a></DataTableTD>
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(p.nhsNumber)}">{p.patientName}</a>
                    </DataTableTD>
                    <DataTableTD>{when(p.enteredAt)}</DataTableTD>
                    <DataTableTD>
                        {#if p.leftAt}{when(p.leftAt)}{:else}<Badge type="success">{t('common.stillHere')}</Badge>{/if}
                    </DataTableTD>
                    <DataTableTD>{p.leftReason ?? ''}</DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.presences.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={5}>{t('cabinetDetail.noPresence')}</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
