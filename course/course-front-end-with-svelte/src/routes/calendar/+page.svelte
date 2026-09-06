<!--
  Instance calendar (`/calendar`) — every course instance's schedule
  window as an all-day span in the SVAR Calendar (month view,
  read-only); individual sessions render as timed events. Selecting
  an entry opens the owning course. Loads courses via search (capped)
  and fans in each course's instances.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { Calendar, Willow } from "@svar-ui/svelte-calendar";
    import type { CalendarInstanceApi } from "@svar-ui/svelte-calendar";
    import { CourseRepository } from "$lib/api/courses";
    import type { Course, CourseInstance } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte.js";

    const repo = CourseRepository.withFetch();

    type Entry = { course: Course; instance: CourseInstance };

    let entries = $state<Entry[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);

    onMount(async () => {
        try {
            const res = await repo.search({ q: "*", limit: 50 });
            const courses = res.items.filter((c) => c.id);
            const instanceLists = await Promise.all(
                courses.map((c) => repo.listInstances(c.id ?? "")),
            );
            const next: Entry[] = [];
            courses.forEach((course, index) => {
                for (const instance of instanceLists[index] ?? []) {
                    next.push({ course, instance });
                }
            });
            entries = next;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Midnight of the same calendar day (drops any time-of-day component).
    function startOfDay(d: Date): Date {
        return new Date(d.getFullYear(), d.getMonth(), d.getDate());
    }

    // The instance's overall window renders as an all-day span; each
    // concrete session as a timed event.
    const events = $derived(
        entries.flatMap(({ course, instance }) => {
            const label = instance.name ?? course.name;
            const out: {
                id: string;
                start: Date;
                end: Date;
                allDay?: boolean;
                text: string;
            }[] = [];
            const schedule = instance.schedule;
            if (schedule?.start_date) {
                const start = new Date(schedule.start_date);
                const end = new Date(schedule.end_date ?? schedule.start_date);
                // The SVAR calendar store drops an all-day event whose
                // `end` is not strictly after `start` (`!(e.end > start)`),
                // which silently loses a single-day window since `end`
                // otherwise equals `start` exactly. Use an exclusive end:
                // one day past the later of the two calendar days.
                const lastDay =
                    startOfDay(end) > startOfDay(start)
                        ? startOfDay(end)
                        : startOfDay(start);
                const nextDay = new Date(
                    lastDay.getFullYear(),
                    lastDay.getMonth(),
                    lastDay.getDate() + 1,
                );
                out.push({
                    id: `${course.id}::${instance.id}::window`,
                    start,
                    end: nextDay,
                    allDay: true,
                    text: label,
                });
            }
            for (const [index, session] of (
                schedule?.sessions ?? []
            ).entries()) {
                out.push({
                    id: `${course.id}::${instance.id}::s${index}`,
                    start: new Date(session.start),
                    end: new Date(session.end ?? session.start),
                    text: session.label ?? label,
                });
            }
            return out;
        }),
    );

    function init(api: CalendarInstanceApi) {
        api.on("select-event", (raw) => {
            const ev = raw as { id: string | number | null };
            const courseId = String(ev.id ?? "").split("::")[0];
            if (courseId) void goto(`/courses/${courseId}`);
        });
    }
</script>

<svelte:head><title>{t("nav.calendar")} — Main X</title></svelte:head>

<h1>{t("nav.calendar")}</h1>

{#if loading}
    <p>{t("detail.loading")}</p>
{:else if error}
    <p class="error" role="alert">{error}</p>
{:else}
    <div class="calendar-wrap" data-testid="instance-calendar">
        <Willow>
            <Calendar {events} view="month" readonly {init} />
        </Willow>
    </div>
{/if}

<style>
    .calendar-wrap {
        height: 640px;
    }
</style>
