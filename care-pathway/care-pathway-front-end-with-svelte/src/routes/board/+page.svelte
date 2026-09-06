<!--
  Instance board (`/board`) — one pathway's enrolled instances as SVAR
  Kanban columns (Active / On hold / Completed / Discontinued). Instances
  across pathways are not directly listable, so the board is built from a
  single selected pathway (a `<select>` seeded from the registry list) via
  `GET /api/care-pathways/{pathway}/instances`. Dragging a card issues
  `POST /api/instances/{pid}/status`; the service's lifecycle machine
  refuses illegal moves (422), and a reload restores the stored truth —
  mirroring the organization review board's drag-to-decide init pattern.

  CPFE-T3: below the board, a "Record a segment" panel wires
  `TbaRepository.recordSegment`/`.setClock` — the repository already had
  these calls, but no page used them, so a journey could never actually
  be mapped by hand from the UI. Reuses the board's own `instances`
  list; literal English labels (not `t()`) match this project's other
  time-based-analysis page, `/time`, which is un-i18n'd for the same
  reason (a data-heavy analytics area, distinct from the CRUD forms).
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
    import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import {
        CATEGORIES,
        STAGES,
        WASTES,
        TbaRepository,
        type Category,
        type SegmentPayload,
        type Stage,
        type Waste,
    } from "$lib/api/tba";
    import {
        INSTANCE_STATUSES,
        type InstanceStatus,
        type PathwayInstance,
        type PathwayRef,
    } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte";

    const repo = CarePathwayRepository.withFetch();
    const tba = TbaRepository.withFetch();

    let refs = $state<PathwayRef[]>([]);
    let selected = $state("");
    let instances = $state<PathwayInstance[] | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let actionError = $state<string | null>(null);
    // Derived caseload context (across all pathways), loaded once.
    let caseload = $state<{ note?: string; total?: number } | null>(null);

    // CPFE-T3: the "Record a segment" panel's own state, independent of
    // the board's drag interactions above.
    let recordingInstance = $state("");
    let segLabel = $state("");
    let segStage = $state<Stage>("treatment");
    let segCategory = $state<Category>("value_adding");
    let segWaste = $state<Waste | "">("");
    let segStartedAt = $state("");
    let segEndedAt = $state("");
    let segActorRef = $state("");
    let segLocationRef = $state("");
    let segNote = $state("");
    let recording = $state(false);
    let recordError = $state<string | null>(null);
    let recordSuccess = $state<string | null>(null);

    // Waste is refused on a value-adding segment and required on an
    // unnecessary one (service rule, `tba.rs::validate_segment_fields`).
    const wasteRequired = $derived(segCategory === "unnecessary_non_value_adding");
    const wasteForbidden = $derived(segCategory === "value_adding");

    async function loadInstances(pathwayPid: string) {
        if (!pathwayPid) {
            instances = [];
            return;
        }
        try {
            instances = await repo.listInstances(pathwayPid);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    onMount(async () => {
        try {
            // Caseload is loaded for operational context (not the board data).
            [refs, caseload] = await Promise.all([
                repo.list(),
                repo.caseload().then((c) => c as { note?: string; total?: number }),
            ]);
            selected = refs[0]?.pid ?? "";
            await loadInstances(selected);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Column ids are the wire tokens (the service serializes the status
    // lowercase / snake_case); labels stay human-cased.
    const columns = [
        { id: "active", label: "Active", addCard: false },
        { id: "on_hold", label: "On hold", addCard: false },
        { id: "completed", label: "Completed", addCard: false },
        { id: "discontinued", label: "Discontinued", addCard: false },
    ];

    const cards = $derived(
        (instances ?? []).map((instance) => ({
            id: instance.pid,
            label: instance.subject_ref,
            description: `${instance.urgency} · enrolled ${instance.enrolled_on}`,
            status: instance.status,
        })),
    );

    // Drag = a lifecycle move through the API. The status machine refuses
    // illegal transitions (422); the reload restores the stored truth.
    function init(api: KanbanInstanceApi) {
        api.on("move-card", (raw) => {
            const ev = raw as { id: string | number; column?: string | number };
            actionError = null;
            const target = String(ev.column ?? "") as InstanceStatus;
            const source = (instances ?? []).find((i) => i.pid === String(ev.id));
            if (
                source &&
                target !== source.status &&
                INSTANCE_STATUSES.includes(target)
            ) {
                void repo
                    .setInstanceStatus(String(ev.id), target)
                    .catch((cause) => {
                        actionError =
                            cause instanceof Error ? cause.message : String(cause);
                    })
                    .finally(() => loadInstances(selected));
            } else {
                void loadInstances(selected);
            }
        });
    }

    // Convert a `datetime-local` input's local-time value ("YYYY-MM-DDTHH:mm")
    // to a full RFC 3339 instant the service's `DateTime<Utc>` field
    // requires; blank input stays blank (an open segment / no clock-stop).
    function toIso(local: string): string | undefined {
        if (!local) return undefined;
        const parsed = new Date(local);
        return Number.isNaN(parsed.getTime()) ? undefined : parsed.toISOString();
    }

    // Record a mapped segment against the currently selected instance
    // (CPFE-T3). Client-side only enforces what the operator can fix
    // before submitting (required fields, the waste/category pairing);
    // the service stays authoritative and answers 422 for anything else.
    async function handleRecordSegment(event: SubmitEvent) {
        event.preventDefault();
        recordError = null;
        recordSuccess = null;
        if (!recordingInstance) {
            recordError = "Select an instance first.";
            return;
        }
        const startedAt = toIso(segStartedAt);
        if (!segLabel.trim() || !startedAt) {
            recordError = "Label and start time are required.";
            return;
        }
        if (wasteRequired && !segWaste) {
            recordError = "Waste is required on an unnecessary non-value-adding segment.";
            return;
        }
        recording = true;
        try {
            const payload: SegmentPayload = {
                label: segLabel.trim(),
                stage: segStage,
                category: segCategory,
                waste: wasteForbidden || !segWaste ? null : segWaste,
                started_at: startedAt,
                ended_at: toIso(segEndedAt) ?? null,
                actor_ref: segActorRef.trim() || null,
                location_ref: segLocationRef.trim() || null,
                note: segNote.trim() || null,
            };
            await tba.recordSegment(recordingInstance, payload);
            recordSuccess = `Segment "${payload.label}" recorded.`;
            segLabel = "";
            segStartedAt = "";
            segEndedAt = "";
            segActorRef = "";
            segLocationRef = "";
            segNote = "";
            segWaste = "";
        } catch (err) {
            recordError = err instanceof Error ? err.message : String(err);
        } finally {
            recording = false;
        }
    }

    // Start or stop the selected instance's clock (CPFE-T3). The clock has
    // no pause by design (`agents/share/time-based-analysis.md`) — only
    // a start and a stop.
    async function handleSetClock(clockEvent: "start" | "stop") {
        recordError = null;
        recordSuccess = null;
        if (!recordingInstance) {
            recordError = "Select an instance first.";
            return;
        }
        recording = true;
        try {
            await tba.setClock(recordingInstance, clockEvent);
            recordSuccess = clockEvent === "start" ? "Clock started." : "Clock stopped.";
        } catch (err) {
            recordError = err instanceof Error ? err.message : String(err);
        } finally {
            recording = false;
        }
    }
</script>

<svelte:head><title>{t("nav.board")} — Main X</title></svelte:head>

<h1>{t("nav.board")}</h1>

<p>
    <label class="locale">
        <span>{t("form.name")}</span>
        <select
            bind:value={selected}
            onchange={() => void loadInstances(selected)}
        >
            {#each refs as ref (ref.pid)}
                <option value={ref.pid}>{ref.name}</option>
            {/each}
        </select>
    </label>
    {#if caseload?.total !== undefined}
        <span class="muted small" data-testid="board-context">
            Caseload: {caseload.total}
        </span>
    {/if}
</p>

{#if loading}
    <p>{t("list.loading")}</p>
{:else if error}
    <p class="banner error" role="alert">{error}</p>
{:else}
    {#if actionError}
        <p class="banner error" role="alert" data-testid="action-error">
            {actionError}
        </p>
    {/if}
    <div class="board-wrap" data-testid="instance-board">
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

    <!-- CPFE-T3: record a mapped journey segment, or start/stop the
         instance's clock, against whichever instance is picked below —
         independent of which one (if any) is selected on the board
         above, since a segment can be recorded without dragging a card. -->
    <section class="surface stack" data-testid="record-segment-panel">
        <h2>Record a segment</h2>
        <label>
            <span>Instance</span>
            <select bind:value={recordingInstance} data-testid="segment-instance">
                <option value="">— choose —</option>
                {#each instances ?? [] as instance (instance.pid)}
                    <option value={instance.pid}
                        >{instance.subject_ref} ({instance.status})</option
                    >
                {/each}
            </select>
        </label>

        <div class="row">
            <button
                type="button"
                onclick={() => handleSetClock("start")}
                disabled={recording || !recordingInstance}
            >
                Start clock
            </button>
            <button
                type="button"
                onclick={() => handleSetClock("stop")}
                disabled={recording || !recordingInstance}
            >
                Stop clock
            </button>
        </div>

        <form class="stack" onsubmit={handleRecordSegment}>
            <label
                >Label
                <input type="text" bind:value={segLabel} required />
            </label>
            <div class="row">
                <label
                    >Stage
                    <select bind:value={segStage}>
                        {#each STAGES as stage (stage)}
                            <option value={stage}>{stage}</option>
                        {/each}
                    </select>
                </label>
                <label
                    >Category
                    <select bind:value={segCategory}>
                        {#each CATEGORIES as category (category)}
                            <option value={category}>{category}</option>
                        {/each}
                    </select>
                </label>
                <label
                    >Waste {wasteRequired ? "(required)" : wasteForbidden ? "(not allowed)" : "(optional)"}
                    <select bind:value={segWaste} disabled={wasteForbidden}>
                        <option value="">—</option>
                        {#each WASTES as waste (waste)}
                            <option value={waste}>{waste}</option>
                        {/each}
                    </select>
                </label>
            </div>
            <div class="row">
                <label
                    >Started at
                    <input type="datetime-local" bind:value={segStartedAt} required />
                </label>
                <label
                    >Ended at <small>(leave blank to open a running segment)</small>
                    <input type="datetime-local" bind:value={segEndedAt} />
                </label>
            </div>
            <div class="row">
                <label
                    >Actor <small>(optional)</small>
                    <input type="text" bind:value={segActorRef} placeholder="worker:…" />
                </label>
                <label
                    >Location <small>(optional)</small>
                    <input type="text" bind:value={segLocationRef} placeholder="place:…" />
                </label>
            </div>
            <label
                >Note <small>(optional)</small>
                <input type="text" bind:value={segNote} />
            </label>
            <button class="button" type="submit" disabled={recording}>
                {recording ? "Recording…" : "Record segment"}
            </button>
        </form>

        {#if recordError}
            <p class="banner error" role="alert" data-testid="record-error">{recordError}</p>
        {/if}
        {#if recordSuccess}
            <p class="banner success" role="status" data-testid="record-success">
                {recordSuccess}
            </p>
        {/if}
    </section>
{/if}

<style>
    .board-wrap {
        height: 560px;
        overflow-x: auto;
    }
</style>
