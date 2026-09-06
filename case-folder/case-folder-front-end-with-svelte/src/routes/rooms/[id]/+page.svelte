<script lang="ts">
    // Room detail (`/rooms/[id]`) — folder presence history for a room.
    //
    // Render-only. Shows the presence timeline aggregated across all of
    // the room's cabinets (newest first); a null `leftAt` ⇒ still present.

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';
    import { t } from '$lib/i18n.svelte';

    let { data } = $props();

    // Format an ISO timestamp in UK locale for the timeline columns.
    function when(iso: string): string {
        return new Date(iso).toLocaleString('en-GB');
    }

    // The patient route is keyed by the bare (spaceless) NHS Number.
    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/buildings">{t('roomDetail.backToBuildings')}</BackLink>

<h2>{data.place.name}</h2>
{#if data.place.container_path}<p>{data.place.container_path}</p>{/if}

<h3>{t('roomDetail.presenceHistory')}</h3>
<p>{t('roomDetail.presenceIntro')}</p>
<div class="panel">
    <DataTable
        label={t('roomDetail.presenceTable')}
        caption={t('roomDetail.presenceCaption')}
    >
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('common.folder')}</th>
                <th scope="col">{t('common.patient')}</th>
                <th scope="col">{t('common.cabinet')}</th>
                <th scope="col">{t('common.entered')}</th>
                <th scope="col">{t('common.left')}</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each data.presences as p (p.cabinetId + p.folderId + p.enteredAt)}
                <DataTableRow>
                    <DataTableTD
                        ><a href="/folders/{p.folderId}">{p.folderTitle}</a
                        ></DataTableTD
                    >
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(p.nhsNumber)}"
                            >{p.patientName}</a
                        >
                    </DataTableTD>
                    <DataTableTD>{p.cabinetLabel}</DataTableTD>
                    <DataTableTD>{when(p.enteredAt)}</DataTableTD>
                    <DataTableTD>
                        {#if p.leftAt}{when(p.leftAt)}{:else}<Badge
                                type="success">{t('common.stillHere')}</Badge
                            >{/if}
                    </DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.presences.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={5}
                        >{t('roomDetail.noPresence')}</DataTableTD
                    >
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
