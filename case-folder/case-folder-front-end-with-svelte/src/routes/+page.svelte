<script lang="ts">
    // Dashboard (`/`) — at-a-glance overview of the folder estate.
    //
    // Reads everything reactively from the cache hydrated by `+page.ts`:
    // headline stats (patients / in-cabinet / in-transit / places /
    // 24h moves), the full folder register grid, the five most recent
    // moves, and per-cabinet utilisation. Render-only; no local state.

    import { cache } from '$lib/store/cache.svelte';

    import Card from '$lib/components/Card/Card.svelte';
    import Banner from '$lib/components/Banner/Banner.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Separator from '$lib/components/Separator/Separator.svelte';
    import SummaryList from '$lib/components/SummaryList/SummaryList.svelte';
    import SummaryListItem from '$lib/components/SummaryListItem/SummaryListItem.svelte';
    import FolderGrid from '$lib/components/FolderGrid.svelte';

    const stats = $derived(cache.stats);
    const folders = $derived(cache.folders);
    const cabinets = $derived(cache.cabinets);
    const moves = $derived(cache.moves);
</script>

{#if stats}
    <Banner type="info">
        <strong>Welcome.</strong>
        You have {stats.folders.inTransit} folder{stats.folders.inTransit === 1 ? '' : 's'} currently in transit. Use the
        <a href="/move">Move folder</a> page to record a placement.
    </Banner>

    <section class="metric-grid" aria-label="Folder summary">
        <Card heading="Patients" headingLevel={3}>
            <p class="metric-value">{stats.patients}</p>
            <p class="metric-sub">{stats.folders.total} folders tracked</p>
        </Card>
        <Card heading="In cabinet" headingLevel={3}>
            <p class="metric-value">{stats.folders.inCabinet}</p>
            <p class="metric-sub"><Badge type="success">Located</Badge></p>
        </Card>
        <Card heading="In transit" headingLevel={3}>
            <p class="metric-value">{stats.folders.inTransit}</p>
            <p class="metric-sub"><Badge type="warning">Porter in motion</Badge></p>
        </Card>
        <Card heading="Buildings" headingLevel={3}>
            <p class="metric-value">{stats.places.buildings}</p>
            <p class="metric-sub">{stats.places.rooms} rooms · {stats.places.cabinets} cabinets</p>
        </Card>
        <Card heading="Moves (24h)" headingLevel={3}>
            <p class="metric-value">{stats.moves24h}</p>
            <p class="metric-sub">Audited folder placements</p>
        </Card>
    </section>
{/if}

<Separator />

<section aria-labelledby="folders-heading">
    <div class="toolbar">
        <h2 id="folders-heading">Folder register</h2>
        <div>
            <a href="/folders" class="button secondary">View all</a>
            <a href="/folders/new" class="button">Add folder</a>
        </div>
    </div>
    <FolderGrid {folders} />
</section>

<Separator />

<div class="split">
    <section class="panel" aria-labelledby="recent-moves">
        <h2 id="recent-moves">Recent moves</h2>
        <div class="move-stack">
            {#each moves.slice(0, 5) as move (move.id)}
                <article class="move-card">
                    <div class="move-route">
                        <strong>{move.folderTitle}</strong>:
                        <span>{move.fromCabinetLabel}</span>
                        <span class="move-arrow" aria-hidden="true">→</span>
                        <span>{move.toCabinetLabel}</span>
                    </div>
                    <p class="move-meta">
                        <span class="nhs-number">{move.nhsNumber}</span>
                        — {move.patientName} · {move.movedBy}
                        · {new Date(move.movedAt).toLocaleString('en-GB')}
                        {#if move.reason}· {move.reason}{/if}
                    </p>
                </article>
            {/each}
        </div>
        <p style="margin-top: var(--nhs-space-3);">
            <a href="/history">See full audit history →</a>
        </p>
    </section>

    <section class="panel" aria-labelledby="cabinet-util">
        <h2 id="cabinet-util">Cabinet utilisation</h2>
        <SummaryList label="Cabinet utilisation">
            {#each cabinets as c (c.id)}
                <!-- Capacity may be null (uncapped); treat as 0 so the
                     percentage maths is skipped and we show ∞ / — instead. -->
                {@const cap = c.capacity ?? 0}
                {@const percent = cap > 0 ? Math.round((c.folderCount / cap) * 100) : 0}
                <SummaryListItem term={c.label}>
                    <span>{c.folderCount} / {cap || '∞'}</span>
                    <!-- Traffic-light fill: red >85%, amber >60%, else green. -->
                    <Badge type={percent > 85 ? 'error' : percent > 60 ? 'warning' : 'success'}>
                        {cap > 0 ? `${percent}%` : '—'}
                    </Badge>
                    <div style="font-size: var(--nhs-font-size-14); color: var(--nhs-dark-grey);">
                        {c.containerPath || '?'}
                    </div>
                </SummaryListItem>
            {/each}
        </SummaryList>
    </section>
</div>
