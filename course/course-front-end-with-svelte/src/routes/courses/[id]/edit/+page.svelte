<!--
  Edit course (route "/courses/[id]/edit") — loads the existing course
  client-side, seeds CourseForm with it, and PUTs the update, returning
  to the detail page on success.

  Reactive state:
    - id ($derived) — route param.
    - course / loading / error — fetch result and status.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import CourseForm from "$lib/components/CourseForm.svelte";
    import { CourseRepository } from "$lib/api/courses.js";
    import type { Course } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    const repo = CourseRepository.withFetch();
    let course = $state<Course | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    const id = $derived(page.params.id as string);

    onMount(async () => {
        try {
            course = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Persist the edited course, then navigate back to its detail page.
    async function handleSubmit(value: Course) {
        await repo.update(id, value);
        goto(`/courses/${id}`);
    }
</script>

<svelte:head><title>Edit course · {id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("edit.title")}</h1>
    <a href={`/courses/${id}`} class="button">{t("edit.cancel")}</a>
</header>

{#if loading}
    <p class="muted">{t("edit.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if course}
    <section class="surface stack">
        <CourseForm initial={course} submitLabel={t("edit.saveChanges")} onsubmit={handleSubmit} />
    </section>
{/if}
