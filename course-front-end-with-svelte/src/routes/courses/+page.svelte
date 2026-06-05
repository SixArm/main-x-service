<script lang="ts">
    import { goto } from "$app/navigation";
    import SearchBox from "$lib/components/SearchBox.svelte";
    import CourseGrid from "$lib/components/CourseGrid.svelte";
    import { CourseRepository } from "$lib/api/courses.js";
    import type { Course } from "$lib/api/types.js";

    let query = $state("");
    let courses = $state<Course[]>([]);
    let total = $state(0);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let fuzzy = $state(true);

    const repo = CourseRepository.withFetch();

    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            // The service falls back to `list` when `q` is empty,
            // which is the "show all" behaviour this page wants on
            // initial load. Sending `"*"` would hit the Tantivy
            // search path with a literal `*` term and return zero
            // hits.
            const res = await repo.search({ q: q.trim(), limit: 50, fuzzy });
            courses = res.items;
            total = res.total;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            courses = [];
            total = 0;
        } finally {
            loading = false;
        }
    }

    function openCourse(course: Course) {
        if (course.id) goto(`/courses/${course.id}`);
    }

    $effect(() => {
        void runSearch("");
    });
</script>

<svelte:head><title>Courses · Course Service</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Courses</h1>
    <a href="/courses/new" class="button primary">New course</a>
</header>

<section class="surface stack">
    <SearchBox bind:value={query} placeholder="Search by name, identifier…" onsearch={runSearch} />
    <div class="row small">
        <label><input type="checkbox" bind:checked={fuzzy} /> Fuzzy</label>
        <span class="muted" style="margin-left: auto">
            {loading ? "Loading…" : `${total} record${total === 1 ? "" : "s"}`}
        </span>
    </div>
    {#if error}
        <div class="banner error">{error}</div>
    {/if}
    <CourseGrid {courses} onselect={openCourse} />
</section>
