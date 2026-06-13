<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { EventRepository } from "$lib/api/events.js";
    import type { Event, Location, Party } from "$lib/api/types.js";

    const repo = EventRepository.withFetch();
    let event = $state<Event | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    const id = $derived(page.params.id as string);

    onMount(async () => {
        try {
            event = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    async function handleDelete() {
        if (!confirm("Soft-delete this event? This cannot be undone via the UI.")) return;
        try {
            await repo.softDelete(id);
            goto("/events");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    function locationLabel(loc: Location): string {
        switch (loc.kind) {
            case "place": return `Place: ${loc.name}${loc.address?.city ? ` (${loc.address.city})` : ""}`;
            case "postal_address": return `Address: ${[loc.line1, loc.city, loc.country].filter(Boolean).join(", ")}`;
            case "virtual": return `Virtual: ${loc.url}`;
            case "text": return `Text: ${loc.value}`;
        }
    }

    function partyLabel(p: Party): string {
        return `${p.name} (${p.kind})${p.email ? ` · ${p.email}` : ""}`;
    }
</script>

<svelte:head><title>Event · {id}</title></svelte:head>

{#if loading}
    <p class="muted">Loading…</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if event}
    <header class="row" style="justify-content: space-between">
        <h1>{event.name}</h1>
        <div class="row">
            <a href={`/events/${id}/edit`} class="button">Edit</a>
            <a href={`/events/${id}/audit`} class="button">Audit</a>
            <button class="button danger" onclick={handleDelete}>Delete</button>
        </div>
    </header>

    <section class="surface stack">
        <h2>Identity</h2>
        <dl class="kv">
            <dt>ID</dt><dd><code>{event.id}</code></dd>
            <dt>Start</dt><dd>{new Date(event.start_date).toLocaleString()}</dd>
            <dt>End</dt><dd>{event.end_date ? new Date(event.end_date).toLocaleString() : "—"}</dd>
            <dt>Status</dt><dd>{event.event_status ?? "—"}</dd>
            <dt>Type</dt><dd>{event.event_type ?? "—"}</dd>
            <dt>Mode</dt><dd>{event.event_attendance_mode ?? "—"}</dd>
            <dt>Time zone</dt><dd>{event.time_zone ?? "—"}</dd>
            <dt>Duration</dt><dd>{event.duration ?? "—"}</dd>
            <dt>Description</dt><dd>{event.description ?? "—"}</dd>
        </dl>
    </section>

    {#if event.location && event.location.length > 0}
        <section class="surface stack">
            <h2>Location</h2>
            <ul>{#each event.location as loc}<li>{locationLabel(loc)}</li>{/each}</ul>
        </section>
    {/if}

    {#if event.organizers && event.organizers.length > 0}
        <section class="surface stack">
            <h2>Organizers</h2>
            <ul>{#each event.organizers as p}<li>{partyLabel(p)}</li>{/each}</ul>
        </section>
    {/if}

    {#if event.performers && event.performers.length > 0}
        <section class="surface stack">
            <h2>Performers</h2>
            <ul>{#each event.performers as p}<li>{partyLabel(p)}</li>{/each}</ul>
        </section>
    {/if}

    {#if event.identifiers && event.identifiers.length > 0}
        <section class="surface stack">
            <h2>Identifiers</h2>
            <ul>
                {#each event.identifiers as identifier}
                    <li>
                        <strong>{identifier.identifier_type}</strong>
                        <code>{identifier.value}</code>
                        <span class="muted small">@ {identifier.system}</span>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if event.offers && event.offers.length > 0}
        <section class="surface stack">
            <h2>Offers</h2>
            <ul>
                {#each event.offers as o}
                    <li>{o.name ?? "Ticket"}: {o.price} {o.price_currency} <span class="muted small">({o.availability ?? "—"})</span></li>
                {/each}
            </ul>
        </section>
    {/if}
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
    ul { margin: 0; padding-left: 1.25rem; }
</style>
