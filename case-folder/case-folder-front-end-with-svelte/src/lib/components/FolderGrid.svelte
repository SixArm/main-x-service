<script lang="ts">
    // FolderGrid wraps SVAR's @svar-ui/svelte-grid for the main dashboard
    // table, wrapped in the Willow theme so it inherits SVAR styling
    // without any CSS file imports (the 2.x release ships styles inside
    // the theme component).
    //
    // Folders carry their NHS Number / patient name / cabinet label
    // already (the API echoes those snapshots), so this component
    // doesn't need any joins.

    import { Grid, Willow } from '@svar-ui/svelte-grid';
    import {
        FilterBar,
        Willow as FilterTheme,
        createArrayFilter,
    } from '@svar-ui/svelte-filter';
    import type { Folder } from '$lib/store/types';
    import { t } from '$lib/i18n.svelte';

    let { folders }: { folders: Folder[] } = $props();

    // Column headers are reactive so a locale switch relabels the grid.
    const columns = $derived([
        { id: 'nhsNumber', header: t('grid.nhsNumber'), width: 130 },
        { id: 'patientName', header: t('grid.patient'), width: 160 },
        { id: 'title', header: t('grid.folder'), width: 200 },
        { id: 'cabinetLabel', header: t('grid.cabinet'), width: 220 },
        { id: 'status', header: t('grid.status'), width: 110 },
        { id: 'lastMovedAt', header: t('grid.lastMoved'), flexgrow: 1 }
    ]);

    let rows = $derived(
        folders.map((f) => ({
            id: f.id,
            nhsNumber: f.nhsNumber,
            patientName: f.patientName,
            title: f.title,
            cabinetLabel: f.cabinetLabel,
            status: f.status,
            lastMovedAt: f.lastMovedAt ? new Date(f.lastMovedAt).toLocaleString('en-GB') : '—'
        }))
    );

    // FilterBar fields over the flattened rows (all columns are
    // human-meaningful here, so every column is filterable).
    const filterFields = $derived(
        columns.map((c) => ({ id: c.id, label: c.header, type: 'text' }))
    );

    // The FilterBar's current rule tree; null = show everything.
    let filterRules = $state<unknown>(null);

    // Rows surviving the filter.
    const filtered = $derived(
        filterRules
            ? createArrayFilter(
                  filterRules as Parameters<typeof createArrayFilter>[0]
              )(rows)
            : rows
    );
</script>

<div class="svar-grid-wrap">
    <Willow>
        <FilterTheme>
            <div class="filter-wrap">
                <FilterBar
                    fields={filterFields}
                    onchange={({ value }: { value: unknown }) => (filterRules = value)}
                />
            </div>
            <Grid data={filtered} {columns} />
        </FilterTheme>
    </Willow>
</div>
