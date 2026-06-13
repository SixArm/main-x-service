<script lang="ts">
    import { goto } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    let { data } = $props();

    let query = $state('');
    $effect.pre(() => {
        query = data.query;
    });
    let debounce: ReturnType<typeof setTimeout> | null = null;

    function onSearchInput() {
        if (debounce) clearTimeout(debounce);
        debounce = setTimeout(() => {
            const next = query.trim();
            const target = next ? `/history?q=${encodeURIComponent(next)}` : '/history';
            goto(target, { keepFocus: true, replaceState: true });
        }, 200);
    }
</script>

<BackLink href="/">Back to dashboard</BackLink>

<div class="toolbar">
    <h2>Move history (audit log)</h2>
    <input
        type="search"
        bind:value={query}
        oninput={onSearchInput}
        placeholder="Filter by patient, NHS number, cabinet, or porter"
        aria-label="Filter audit log"
    />
</div>

<div class="panel">
    <DataTable label="Move audit log" caption="Every recorded movement of a paper folder, newest first">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">When</th>
                <th scope="col">NHS Number</th>
                <th scope="col">Patient</th>
                <th scope="col">From</th>
                <th scope="col">To</th>
                <th scope="col">Moved by</th>
                <th scope="col">Reason</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each cache.moves as move (move.id)}
                <DataTableRow>
                    <DataTableTD>
                        <a href="/history/{move.id}">{new Date(move.movedAt).toLocaleString('en-GB')}</a>
                    </DataTableTD>
                    <DataTableTD>
                        <a href="/patients/{move.nhsNumber.replaceAll(' ', '')}" class="nhs-number">
                            {move.nhsNumber}
                        </a>
                    </DataTableTD>
                    <DataTableTD>{move.patientName}</DataTableTD>
                    <DataTableTD>{move.fromCabinetLabel}</DataTableTD>
                    <DataTableTD>{move.toCabinetLabel}</DataTableTD>
                    <DataTableTD>
                        {move.movedBy}{#if move.workerRole} ({move.workerRole}){/if}
                    </DataTableTD>
                    <DataTableTD>{move.reason ?? ''}</DataTableTD>
                </DataTableRow>
            {/each}
            {#if cache.moves.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={7}>No moves match your filter.</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
