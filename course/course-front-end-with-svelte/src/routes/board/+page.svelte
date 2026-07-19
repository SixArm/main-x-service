<!--
  Lifecycle board (`/board`) — courses as SVAR Kanban cards, one
  column per lifecycle status (draft / published / archived /
  retired). Dragging a card writes the status change via the normal
  full-record PUT, then reloads the truth.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
    import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
    import { CourseRepository } from "$lib/api/courses";
    import { COURSE_STATUSES } from "$lib/api/types";
    import type { Course, CourseStatus } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte.js";

    const repo = CourseRepository.withFetch();

    let courses = $state<Course[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);

    async function load() {
        try {
            const res = await repo.search({ q: "*", limit: 100 });
            courses = res.items;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    }

    onMount(load);

    const columns = COURSE_STATUSES.map((status) => ({
        id: status,
        label: status,
        addCard: false,
    }));

    const cards = $derived(
        courses
            .filter((course) => course.id && course.status)
            .map((course) => ({
                id: course.id ?? "",
                label: course.name,
                description: course.course_code ?? "",
                status: course.status ?? "draft",
            })),
    );

    // Drag = a lifecycle change via the full-record PUT; the reload
    // puts the card back where the truth says it belongs on failure.
    function init(api: KanbanInstanceApi) {
        api.on("move-card", (raw) => {
            const ev = raw as { id: string | number; column?: string | number };
            const course = courses.find((c) => c.id === String(ev.id));
            if (!course || !ev.column) return;
            const updated: Course = {
                ...course,
                status: String(ev.column) as CourseStatus,
            };
            void repo
                .update(String(ev.id), updated)
                .catch((err) => {
                    error = err instanceof Error ? err.message : String(err);
                })
                .finally(load);
        });
    }
</script>

<svelte:head><title>{t("nav.board")} — Main X</title></svelte:head>

<h1>{t("nav.board")}</h1>

{#if loading}
    <p>{t("detail.loading")}</p>
{:else if error}
    <p class="error" role="alert">{error}</p>
{:else}
    <div class="board-wrap" data-testid="course-board">
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
{/if}

<style>
    .board-wrap {
        height: 560px;
        overflow-x: auto;
    }
</style>
