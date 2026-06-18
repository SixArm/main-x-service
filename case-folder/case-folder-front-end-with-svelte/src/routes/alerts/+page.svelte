<script lang="ts">
    // Alerts (`/alerts`) — geofence-breach log.
    //
    // Render-only. Lists moves whose origin and destination cabinets sit
    // in different buildings (a folder leaving its building), newest first.

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Icon from '$lib/components/Icon/Icon.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';
    import { t } from '$lib/i18n.svelte';

    let { data } = $props();

    // The patient route is keyed by the bare (spaceless) NHS Number.
    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/">{t('alerts.backToDashboard')}</BackLink>

<h2><Icon name="scales" /> {t('alerts.heading')}</h2>
<p>{t('alerts.intro')}</p>

<div class="panel">
    <DataTable label={t('alerts.tableLabel')} caption={t('alerts.tableCaption')}>
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('common.when')}</th>
                <th scope="col">{t('common.folder')}</th>
                <th scope="col">{t('common.patient')}</th>
                <th scope="col">{t('alerts.colCrossed')}</th>
                <th scope="col">{t('common.movedBy')}</th>
                <th scope="col">{t('common.reason')}</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each data.alerts as alert (alert.moveId)}
                <DataTableRow>
                    <DataTableTD>
                        <a href="/history/{alert.moveId}">{new Date(alert.movedAt).toLocaleString('en-GB')}</a>
                    </DataTableTD>
                    <DataTableTD><a href="/folders/{alert.folderId}">{alert.folderTitle}</a></DataTableTD>
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(alert.nhsNumber)}">{alert.patientName}</a>
                    </DataTableTD>
                    <DataTableTD>
                        <Badge type="warning">{alert.fromBuilding} → {alert.toBuilding}</Badge>
                    </DataTableTD>
                    <DataTableTD>{alert.movedBy}</DataTableTD>
                    <DataTableTD>{alert.reason ?? ''}</DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.alerts.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={6}>{t('alerts.none')}</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
