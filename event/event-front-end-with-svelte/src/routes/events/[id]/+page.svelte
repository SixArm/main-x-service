<!--
  Event detail page (route "/events/[id]") — loads one event by id and
  renders its identity, locations, parties, identifiers, and offers, with
  edit/audit/delete actions. A masked-view toggle (T-19) re-fetches
  through GET /api/events/{id}/masked instead of the plain record.

  State ($state): the loaded event, error, and loading flag.
    - masked — whether the masked view is currently shown; re-fetches on
      toggle rather than masking client-side, so this always reflects
      the server's actual masking rules.
  Derived ($derived): `id` read from the route param.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { EventRepository } from "$lib/api/events.js";
    import { t, translate } from "$lib/i18n.svelte.js";
    import type { Event, Location, Party } from "$lib/api/types.js";

    const repo = EventRepository.withFetch();
    let event = $state<Event | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);
    let exporting = $state(false);
    let masked = $state(false);

    // Route param identifying which event to display.
    const id = $derived(page.params.id as string);

    // Fetch the plain or masked record depending on `masked`, replacing
    // whatever is currently shown. Shared by the initial load and the
    // toggle handler so both go through one code path.
    async function load() {
        loading = true;
        error = null;
        try {
            event = masked ? await repo.masked(id) : await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    }

    // Flip the toggle and re-fetch through the new endpoint (T-19). A
    // dedicated request per view, not client-side redaction — the
    // server, not this page, decides what counts as sensitive.
    function toggleMasked() {
        masked = !masked;
        void load();
    }

    onMount(load);

    // Soft-delete after a confirm prompt, then return to the list.
    async function handleDelete() {
        if (!confirm(translate("detail.confirmDelete"))) return;
        try {
            await repo.softDelete(id);
            goto("/events");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    // Render a one-line label for any Location variant by switching on `kind`
    // (exhaustive over the discriminated union).
    function locationLabel(loc: Location): string {
        switch (loc.kind) {
            case "place":
                return `${translate("detail.loc.place")}: ${loc.name}${loc.address?.city ? ` (${loc.address.city})` : ""}`;
            case "postal_address":
                return `${translate("detail.loc.address")}: ${[loc.line1, loc.city, loc.country].filter(Boolean).join(", ")}`;
            case "virtual":
                return `${translate("detail.loc.virtual")}: ${loc.url}`;
            case "text":
                return `${translate("detail.loc.text")}: ${loc.value}`;
        }
    }

    // Render a one-line label for a Party (name, kind, optional email).
    function partyLabel(p: Party): string {
        return `${p.name} (${p.kind})${p.email ? ` · ${p.email}` : ""}`;
    }
    // GDPR export (T-20): fetch the service's export payload and hand it
    // to the browser as a downloaded JSON file — the payload shape is
    // service-defined (`exportGdpr` returns `unknown`), so this never
    // interprets it, only serializes and saves what came back. A Blob
    // object URL through a synthetic anchor is the plain-browser way to
    // save client-held data; the URL is revoked once the click has fired.
    async function handleExportGdpr() {
        exporting = true;
        error = null;
        try {
            const data = await repo.exportGdpr(id);
            const blob = new Blob([JSON.stringify(data, null, 2)], {
                type: "application/json",
            });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = `event-${id}-export.json`;
            document.body.appendChild(a);
            a.click();
            a.remove();
            URL.revokeObjectURL(url);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            exporting = false;
        }
    }
</script>

<svelte:head><title>Event · {id}</title></svelte:head>

{#if loading}
    <p class="muted">{t("detail.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if event}
    <header class="row" style="justify-content: space-between">
        <h1>{event.name}</h1>
        <div class="row">
            <button class="button" aria-pressed={masked} onclick={toggleMasked}>
                {masked ? t("detail.showFull") : t("detail.showMasked")}
            </button>
            <a href={`/events/${id}/edit`} class="button">{t("detail.edit")}</a>
            <a href={`/events/${id}/audit`} class="button"
                >{t("detail.audit")}</a
            >
            <button
                class="button"
                onclick={handleExportGdpr}
                disabled={exporting}
            >
                {exporting ? t("detail.exportingGdpr") : t("detail.exportGdpr")}
            </button>
            <button class="button danger" onclick={handleDelete}
                >{t("detail.delete")}</button
            >
        </div>
    </header>

    {#if masked}
        <div class="banner" role="status">{t("detail.maskedNotice")}</div>
    {/if}

    <section class="surface stack">
        <h2>{t("detail.identity")}</h2>
        <dl class="kv">
            <dt>{t("detail.id")}</dt>
            <dd><code>{event.id}</code></dd>
            <dt>{t("detail.start")}</dt>
            <dd>{new Date(event.start_date).toLocaleString()}</dd>
            <dt>{t("detail.end")}</dt>
            <dd>
                {event.end_date
                    ? new Date(event.end_date).toLocaleString()
                    : t("detail.empty")}
            </dd>
            <dt>{t("detail.status")}</dt>
            <dd>{event.event_status ?? t("detail.empty")}</dd>
            <dt>{t("detail.type")}</dt>
            <dd>{event.event_type ?? t("detail.empty")}</dd>
            <dt>{t("detail.mode")}</dt>
            <dd>{event.event_attendance_mode ?? t("detail.empty")}</dd>
            <dt>{t("detail.timeZone")}</dt>
            <dd>{event.time_zone ?? t("detail.empty")}</dd>
            <dt>{t("detail.duration")}</dt>
            <dd>{event.duration ?? t("detail.empty")}</dd>
            <dt>{t("detail.description")}</dt>
            <dd>{event.description ?? t("detail.empty")}</dd>
        </dl>
    </section>

    {#if event.location && event.location.length > 0}
        <section class="surface stack">
            <h2>{t("detail.location")}</h2>
            <ul>
                {#each event.location as loc}<li>
                        {locationLabel(loc)}
                    </li>{/each}
            </ul>
        </section>
    {/if}

    {#if event.organizers && event.organizers.length > 0}
        <section class="surface stack">
            <h2>{t("detail.organizers")}</h2>
            <ul>
                {#each event.organizers as p}<li>{partyLabel(p)}</li>{/each}
            </ul>
        </section>
    {/if}

    {#if event.performers && event.performers.length > 0}
        <section class="surface stack">
            <h2>{t("detail.performers")}</h2>
            <ul>
                {#each event.performers as p}<li>{partyLabel(p)}</li>{/each}
            </ul>
        </section>
    {/if}

    {#if event.identifiers && event.identifiers.length > 0}
        <section class="surface stack">
            <h2>{t("detail.identifiers")}</h2>
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
            <h2>{t("detail.offers")}</h2>
            <ul>
                {#each event.offers as o}
                    <li>
                        {o.name ?? t("detail.ticket")}: {o.price}
                        {o.price_currency}
                        <span class="muted small"
                            >({o.availability ?? t("detail.empty")})</span
                        >
                    </li>
                {/each}
            </ul>
        </section>
    {/if}
{/if}

<style>
    .kv {
        display: grid;
        grid-template-columns: max-content 1fr;
        column-gap: 1rem;
        row-gap: 0.25rem;
    }
    dt {
        font-weight: 600;
    }
    dd {
        margin: 0;
    }
    ul {
        margin: 0;
        padding-left: 1.25rem;
    }
</style>
