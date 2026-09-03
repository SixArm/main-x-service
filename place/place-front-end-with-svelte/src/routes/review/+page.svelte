<!--
  Duplicate-review board (`/review`) — the STORED review queue, presented
  three ways over one selection:

    1. A SVAR Kanban board (Pending / Confirmed / Rejected / AutoMerged).
       Dragging a pending card records the decision; clicking one opens
       the comparison panel.
    2. A native table of the same items. This is the keyboard path: the
       board's drag interaction cannot be driven from a keyboard, so every
       row carries a real `Compare` button and the panel carries real
       `Confirm` / `Reject` buttons.
    3. An inline comparison panel below both, fetching each side of the
       pair with its own `GET /api/places/{id}` (there is no combined
       "fetch the pair" endpoint) and rendering the matcher's per-
       component score breakdown.

  The queue loads on mount and whenever the status/page-size filter
  changes (all safe GETs). The batch scan runs only on the button —
  `POST /deduplicate` is a destructive-classed action, never a page-load
  side effect — and upserts into the same stored queue with stable ids.

  Confirming does NOT merge: the decision endpoint is a pure status
  change and the service records no link between a confirmed item and a
  merge. The panel therefore deep-links to `/places/merge` with both ids
  pre-filled, in either survivor order, since a review item names an
  unordered pair.

  The service's first-writer-wins guard refuses non-pending transitions,
  so `canDecide` disables the buttons rather than offering a request that
  is guaranteed to answer 422.

  KNOWN GAP vs. person's review screen (see `$lib/review`'s module doc):
  place-service's `ReviewQueueItem` carries neither `provenance` (no such
  column in `review_queue`) nor a wire-serialized `score_breakdown` (the
  column exists but is always written `NULL` and never serialized). The
  queue therefore surfaces `detection_method` (which IS on the wire)
  instead of a provenance badge, and the breakdown section always renders
  its "not recorded" note today — both are documented, deliberate, and
  covered by tests rather than silently missing.
-->
<script lang="ts">
    import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
    import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
    import { PlaceRepository } from "$lib/api/places";
    import type { ReviewQueueOptions } from "$lib/api/places";
    import type {
        Place,
        ReviewDecision,
        ReviewQueueItem,
        ReviewStatus,
    } from "$lib/api/types";
    import {
        REVIEW_LIMITS,
        REVIEW_STATUSES,
        breakdownRows,
        canDecide,
        mergeHref,
    } from "$lib/review";
    import { t, translate } from "$lib/i18n.svelte.js";
    import type { StringKey } from "$lib/i18n.svelte";

    const repo = PlaceRepository.withFetch();

    let items = $state<ReviewQueueItem[] | null>(null);
    let scanned = $state<number | null>(null);
    let running = $state(false);
    let error = $state<string | null>(null);
    let actionError = $state<string | null>(null);

    // ─── Filters ────────────────────────────────────────────────────
    // `all` is the *absence* of the `status` parameter — the endpoint has
    // no "all" token and answers 422 for one it does not recognise.
    let statusFilter = $state<ReviewStatus | "all">("all");
    let limit = $state<number>(100);

    const query = $derived<ReviewQueueOptions>({
        status: statusFilter === "all" ? undefined : statusFilter,
        limit,
    });

    async function load(options: ReviewQueueOptions = query) {
        error = null;
        try {
            items = await repo.listReviewQueue(options);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    // Reads `query` synchronously, so changing either filter refetches.
    $effect(() => {
        void load(query);
    });

    async function runScan() {
        running = true;
        error = null;
        try {
            const report = await repo.deduplicate({});
            scanned = report.places_scanned;
            // Re-read through the filter rather than trusting the scan's
            // own list, which is unfiltered and only covers this pass.
            await load();
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            running = false;
        }
    }

    // ─── Selection + the comparison panel ───────────────────────────
    //
    // The selected item is held as its own copy rather than derived from
    // `items`, so that deciding it — which may drop it out of the current
    // filter — leaves the panel open on the decided item (and therefore
    // on its merge deep-link).
    let selectedItem = $state<ReviewQueueItem | null>(null);
    let pair = $state<{ a: Place | null; b: Place | null } | null>(null);
    let pairLoading = $state(false);
    let pairPartial = $state(false);
    let deciding = $state(false);
    let panelEl = $state<HTMLElement | null>(null);

    // Discards a slow response whose selection has since been replaced.
    let pairToken = 0;
    // Plain (non-reactive) so the focus effect below cannot re-trigger
    // itself by writing state it also reads.
    let focusedFor: string | null = null;

    async function select(id: string) {
        const item = (items ?? []).find((i) => i.id === id);
        if (!item) return;
        selectedItem = item;
        actionError = null;
        await loadPair(item);
    }

    // Two parallel single-record GETs — there is no combined pair
    // endpoint. `allSettled`, not `all`: one side may be soft-deleted (a
    // 404) and a half-rendered comparison still beats none.
    async function loadPair(item: ReviewQueueItem) {
        const token = ++pairToken;
        pair = null;
        pairPartial = false;
        pairLoading = true;
        const [a, b] = await Promise.allSettled([
            repo.get(item.place_id_a),
            repo.get(item.place_id_b),
        ]);
        if (token !== pairToken) return;
        pair = {
            a: a.status === "fulfilled" ? a.value : null,
            b: b.status === "fulfilled" ? b.value : null,
        };
        pairPartial = a.status === "rejected" || b.status === "rejected";
        pairLoading = false;
    }

    function closePanel() {
        pairToken++;
        selectedItem = null;
        pair = null;
        pairPartial = false;
    }

    // Move focus to the panel when a *different* item opens it, so the
    // keyboard path lands where the new content is — but not on every
    // re-render, which would steal focus back after a decision.
    $effect(() => {
        const item = selectedItem;
        if (item && panelEl && focusedFor !== item.id) {
            focusedFor = item.id;
            panelEl.focus();
        }
        if (!item) focusedFor = null;
    });

    // The keyboard-accessible decision path (the drag path is below).
    async function decide(status: ReviewDecision) {
        const item = selectedItem;
        if (!item || !canDecide(item) || deciding) return;
        deciding = true;
        actionError = null;
        try {
            // The endpoint returns the decided item, so the panel can show
            // the new status without waiting for the list to come back.
            selectedItem = await repo.decideReview(item.id, status);
        } catch (err) {
            actionError = err instanceof Error ? err.message : String(err);
        } finally {
            deciding = false;
            await load();
        }
    }

    // ─── Presentation helpers ───────────────────────────────────────

    const STATUS_LABEL: Record<ReviewStatus, StringKey> = {
        pending: "review.status.pending",
        confirmed: "review.status.confirmed",
        rejected: "review.status.rejected",
        automerged: "review.status.automerged",
    };

    function statusLabel(status: ReviewStatus): string {
        return t(STATUS_LABEL[status]);
    }

    function shortId(id: string): string {
        return id.slice(0, 8);
    }

    function pairLabel(item: ReviewQueueItem): string {
        return `${shortId(item.place_id_a)} ↔ ${shortId(item.place_id_b)}`;
    }

    /** Display name, or `null` when the record is absent. */
    function nameOf(place: Place | null): string | null {
        return place?.name && place.name.trim().length > 0 ? place.name : null;
    }

    /** `place_type` as a human string; `null` when absent. */
    function placeTypeOf(place: Place | null): string | null {
        if (!place?.place_type) return null;
        return typeof place.place_type === "string"
            ? place.place_type
            : translate("detail.typeOther").replace(
                  "{value}",
                  place.place_type.Other,
              );
    }

    /** Flattened postal address; `null` when there is none. */
    function addressLine(place: Place | null): string | null {
        const address = place?.address;
        if (!address) return null;
        const parts = [
            address.street_address,
            address.address_locality,
            address.address_region,
            address.postal_code,
            address.address_country,
        ].filter((part): part is string => !!part && part.trim().length > 0);
        return parts.length > 0 ? parts.join(", ") : null;
    }

    /** `"lat, lon"`; `null` when there is no geo. */
    function geoLine(place: Place | null): string | null {
        const geo = place?.geo;
        return geo
            ? `${geo.latitude_as_decimal_degrees}, ${geo.longitude_as_decimal_degrees}`
            : null;
    }

    /** Renders a possibly-absent field as an explicit "not recorded". */
    function orNone(value: string | null | undefined): string {
        return value && value.trim().length > 0
            ? value
            : t("review.compare.none");
    }

    // The comparison rows, so the table body stays declarative.
    const compareRows = $derived(
        pair
            ? [
                  {
                      labelKey: "detail.id" as StringKey,
                      a: pair.a?.id ?? null,
                      b: pair.b?.id ?? null,
                  },
                  {
                      labelKey: "form.name" as StringKey,
                      a: nameOf(pair.a),
                      b: nameOf(pair.b),
                  },
                  {
                      labelKey: "detail.type" as StringKey,
                      a: placeTypeOf(pair.a),
                      b: placeTypeOf(pair.b),
                  },
                  {
                      labelKey: "detail.address" as StringKey,
                      a: addressLine(pair.a),
                      b: addressLine(pair.b),
                  },
                  {
                      labelKey: "detail.geo" as StringKey,
                      a: geoLine(pair.a),
                      b: geoLine(pair.b),
                  },
                  {
                      labelKey: "detail.telephone" as StringKey,
                      a: pair.a?.telephone ?? null,
                      b: pair.b?.telephone ?? null,
                  },
                  {
                      labelKey: "detail.gln" as StringKey,
                      a: pair.a?.global_location_number ?? null,
                      b: pair.b?.global_location_number ?? null,
                  },
              ]
            : [],
    );

    // Always empty today — see the module doc + `$lib/review`'s KNOWN GAP
    // note: place-service never serializes `score_breakdown` on the wire.
    // Kept wired (not hardcoded away) so the panel activates automatically
    // the day the service starts sending it.
    const breakdown = $derived(
        selectedItem
            ? breakdownRows(
                  (selectedItem as unknown as { score_breakdown?: unknown })
                      .score_breakdown,
              )
            : [],
    );

    // ─── The Kanban board ───────────────────────────────────────────

    // Column ids are the wire tokens (the service serializes the review
    // status lowercase); labels are translated.
    const columns = $derived(
        REVIEW_STATUSES.map((status) => ({
            id: status,
            label: statusLabel(status),
            addCard: false,
        })),
    );

    const cards = $derived(
        (items ?? []).map((item) => ({
            id: item.id,
            label: pairLabel(item),
            description: `${item.match_quality} · ${item.match_score.toFixed(2)} · ${item.detection_method}`,
            status: item.status,
        })),
    );

    // Drag = a review decision through the API. Only pending → confirmed
    // / rejected is a legal move; anything else is refused client-side and
    // the reload restores the stored truth. Click = open the comparison.
    function init(api: KanbanInstanceApi) {
        api.on("move-card", (raw) => {
            const ev = raw as { id: string | number; column?: string | number };
            actionError = null;
            const target = String(ev.column ?? "");
            const source = (items ?? []).find((i) => i.id === String(ev.id));
            if (
                source &&
                canDecide(source) &&
                (target === "confirmed" || target === "rejected")
            ) {
                void repo
                    .decideReview(String(ev.id), target)
                    .catch((cause) => {
                        actionError =
                            cause instanceof Error
                                ? cause.message
                                : String(cause);
                    })
                    .finally(load);
            } else {
                void load();
            }
        });
        api.on("select-card", (raw) => {
            const ev = raw as { id?: string | number | null };
            // The editor clears the selection with `{id: null}`.
            if (ev.id === null || ev.id === undefined) return;
            void select(String(ev.id));
        });
    }
</script>

<svelte:head><title>{t("nav.review")} — Main X</title></svelte:head>

<h1>{t("nav.review")}</h1>

<p class="muted">{t("review.intro")}</p>

<section class="surface stack filters">
    <div class="row">
        <label for="review-status">{t("review.filter.status")}</label>
        <select id="review-status" bind:value={statusFilter}>
            <option value="all">{t("review.filter.statusAll")}</option>
            {#each REVIEW_STATUSES as status (status)}
                <option value={status}>{statusLabel(status)}</option>
            {/each}
        </select>

        <label for="review-limit">{t("review.filter.limit")}</label>
        <select id="review-limit" bind:value={limit}>
            {#each REVIEW_LIMITS as size (size)}
                <option value={size}>{size}</option>
            {/each}
        </select>

        <button onclick={() => void runScan()} disabled={running}>
            {t("review.run")}
        </button>
        {#if scanned !== null}
            <span class="muted">{scanned} · {(items ?? []).length}</span>
        {/if}
    </div>
    <p class="muted hint">{t("review.filter.limitHint")}</p>
</section>

{#if error}
    <p class="banner error" role="alert">{error}</p>
{:else if items === null}
    <p class="muted">{t("review.loading")}</p>
{:else}
    {#if actionError}
        <p class="banner error" role="alert" data-testid="action-error">
            {actionError}
        </p>
    {/if}

    <h2>{t("review.board.title")}</h2>
    <div class="board-wrap" data-testid="review-board">
        <Willow>
            <Kanban
                {cards}
                {columns}
                columnAccessor="status"
                card={{ ...getCardShape(), menu: false }}
                {init}
            />
        </Willow>
    </div>

    <!--
      The keyboard-reachable index of the same items. The board above is
      drag-driven and mouse-only; this is not decoration.
    -->
    <h2>{t("review.list.title")}</h2>
    {#if items.length === 0}
        <p class="muted" data-testid="review-empty">{t("review.empty")}</p>
    {:else}
        <table class="queue" data-testid="review-list">
            <thead>
                <tr>
                    <th scope="col">{t("review.col.pair")}</th>
                    <th scope="col">{t("review.col.score")}</th>
                    <th scope="col">{t("review.col.quality")}</th>
                    <th scope="col">{t("review.col.method")}</th>
                    <th scope="col">{t("review.col.status")}</th>
                    <th scope="col">{t("review.col.actions")}</th>
                </tr>
            </thead>
            <tbody>
                {#each items as item (item.id)}
                    <tr class:selected={selectedItem?.id === item.id}>
                        <td><code>{pairLabel(item)}</code></td>
                        <td>{item.match_score.toFixed(2)}</td>
                        <td>{item.match_quality}</td>
                        <td>
                            <span class="badge">{item.detection_method}</span>
                        </td>
                        <td>{statusLabel(item.status)}</td>
                        <td>
                            <button
                                type="button"
                                onclick={() => void select(item.id)}
                            >
                                {t("review.compare.open")}
                                <span class="sr-only">{pairLabel(item)}</span>
                            </button>
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
{/if}

{#if selectedItem}
    {@const item = selectedItem}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <section
        class="surface stack panel"
        data-testid="review-compare"
        aria-labelledby="review-compare-title"
        tabindex="-1"
        bind:this={panelEl}
    >
        <div class="row panel-head">
            <h2 id="review-compare-title">{t("review.compare.title")}</h2>
            <button type="button" onclick={closePanel}>
                {t("review.compare.close")}
            </button>
        </div>

        <dl class="kv">
            <dt>{t("review.field.score")}</dt>
            <dd>{item.match_score.toFixed(2)}</dd>
            <dt>{t("review.field.quality")}</dt>
            <dd>{item.match_quality}</dd>
            <dt>{t("review.field.method")}</dt>
            <dd>{item.detection_method}</dd>
            <dt>{t("review.field.status")}</dt>
            <dd>{statusLabel(item.status)}</dd>
        </dl>

        {#if pairLoading}
            <p class="muted">{t("review.compare.loading")}</p>
        {:else if pair}
            {#if pairPartial}
                <p class="banner" role="status">
                    {t("review.compare.partial")}
                </p>
            {/if}
            <table class="compare" data-testid="review-compare-table">
                <thead>
                    <tr>
                        <th scope="col">{t("review.compare.field")}</th>
                        <th scope="col">
                            {t("review.compare.a")} · {shortId(item.place_id_a)}
                        </th>
                        <th scope="col">
                            {t("review.compare.b")} · {shortId(item.place_id_b)}
                        </th>
                    </tr>
                </thead>
                <tbody>
                    {#each compareRows as row (row.labelKey)}
                        <tr>
                            <th scope="row">{t(row.labelKey)}</th>
                            <td>{orNone(row.a)}</td>
                            <td>{orNone(row.b)}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}

        <h3>{t("review.breakdown.title")}</h3>
        {#if breakdown.length === 0}
            <p class="muted" data-testid="review-no-breakdown">
                {t("review.breakdown.none")}
            </p>
        {:else}
            <table class="compare" data-testid="review-breakdown">
                <thead>
                    <tr>
                        <th scope="col">{t("review.breakdown.component")}</th>
                        <th scope="col">{t("review.breakdown.weight")}</th>
                        <th scope="col">{t("review.breakdown.score")}</th>
                    </tr>
                </thead>
                <tbody>
                    {#each breakdown as row (row.key)}
                        <tr>
                            <th scope="row">{t(row.labelKey)}</th>
                            <td>{row.weight.toFixed(2)}</td>
                            <td>{row.score.toFixed(2)}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}

        <div class="row">
            <button
                type="button"
                class="button primary"
                disabled={!canDecide(item) || deciding}
                onclick={() => void decide("confirmed")}
            >
                {deciding
                    ? t("review.decide.deciding")
                    : t("review.decide.confirm")}
            </button>
            <button
                type="button"
                class="button"
                disabled={!canDecide(item) || deciding}
                onclick={() => void decide("rejected")}
            >
                {t("review.decide.reject")}
            </button>
        </div>
        {#if !canDecide(item)}
            <p class="muted">{t("review.decide.locked")}</p>
        {/if}

        {#if item.status === "confirmed"}
            <h3>{t("review.merge.title")}</h3>
            <p class="muted">{t("review.merge.note")}</p>
            <div class="row">
                <a
                    class="button"
                    href={mergeHref(item.place_id_a, item.place_id_b)}
                >
                    {t("review.merge.keepA")}
                </a>
                <a
                    class="button"
                    href={mergeHref(item.place_id_b, item.place_id_a)}
                >
                    {t("review.merge.keepB")}
                </a>
            </div>
        {/if}
    </section>
{/if}

<style>
    .board-wrap {
        height: 560px;
        overflow-x: auto;
    }
    .hint {
        margin: 0;
        font-size: 0.85em;
    }
    table.queue,
    table.compare {
        width: 100%;
        border-collapse: collapse;
        margin-block: 0.5rem;
    }
    table.queue th,
    table.queue td,
    table.compare th,
    table.compare td {
        text-align: start;
        padding: 0.35rem 0.5rem;
        border-block-end: 1px solid currentColor;
        vertical-align: top;
    }
    tr.selected {
        font-weight: 600;
    }
    .badge {
        display: inline-block;
        padding: 0.1rem 0.4rem;
        border: 1px solid currentColor;
        border-radius: 0.5rem;
        font-size: 0.8em;
    }
    .panel {
        margin-block-start: 1rem;
    }
    .panel-head {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: 1rem;
    }
    .kv {
        display: grid;
        grid-template-columns: max-content 1fr;
        column-gap: 1rem;
        row-gap: 0.25rem;
    }
    .kv dt {
        font-weight: 600;
    }
    .kv dd {
        margin: 0;
    }
    .sr-only {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }
</style>
