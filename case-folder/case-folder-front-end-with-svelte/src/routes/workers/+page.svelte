<script lang="ts">
    // Workers index (`/workers`) — staff who move folders.
    //
    // Render-only. Lists workers (mirrored from the central Worker
    // Service) with their role; each links to a per-worker activity page.

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';
    import { t } from '$lib/i18n.svelte';

    let { data } = $props();
</script>

<BackLink href="/">{t('common.backToDashboard')}</BackLink>

<h2>{t('workers.heading')}</h2>
<p>{t('workers.intro')}</p>

<div class="panel">
    <DataTable label={t('workers.tableLabel')} caption={t('workers.tableCaption')}>
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('common.name')}</th>
                <th scope="col">{t('common.role')}</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each data.workers as worker (worker.id)}
                <DataTableRow>
                    <DataTableTD><a href="/workers/{worker.id}">{worker.name}</a></DataTableTD>
                    <DataTableTD>{worker.role ?? '—'}</DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.workers.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={2}>{t('workers.noWorkers')}</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
