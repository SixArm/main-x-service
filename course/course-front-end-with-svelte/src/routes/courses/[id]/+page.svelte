<!--
  Course detail (route "/courses/[id]") — read-only view of one course:
  identity fields, identifiers, teaches/keywords/alternate-names,
  same-as URLs, and instances. Offers Edit / Audit links and a
  soft-delete action. Loads the course client-side on mount.

  Reactive state:
    - id ($derived) — route param.
    - course / loading / error — fetch result and status.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { CourseRepository } from "$lib/api/courses.js";
    import type { Course, CourseInstance } from "$lib/api/types.js";
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

    // Confirm, then soft-delete and return to the list on success.
    async function handleDelete() {
        if (!confirm(t("detail.confirmDelete"))) return;
        try {
            await repo.softDelete(id);
            goto("/courses");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    // Display label for an identifier scheme (handles { Custom }).
    function identifierLabel(i: NonNullable<Course["identifiers"]>[number]): string {
        return typeof i.property_id === "string"
            ? i.property_id
            : `${t("detail.customPrefix")} ${i.property_id.Custom}`;
    }

    // Display label for educational level; "—" when unset.
    function levelLabel(c: Course): string {
        const l = c.educational_level;
        if (!l) return t("detail.empty");
        return typeof l === "string" ? l : `${t("detail.customPrefix")} ${l.Custom}`;
    }

    // Headline for an instance: prefer its name, else "date · mode".
    function instanceLabel(inst: CourseInstance): string {
        if (inst.name) return inst.name;
        const when = inst.schedule?.start_date ? new Date(inst.schedule.start_date).toLocaleDateString() : t("detail.noDate");
        return `${when} · ${inst.course_mode ?? t("detail.empty")}`;
    }
</script>

<svelte:head><title>Course · {id}</title></svelte:head>

{#if loading}
    <p class="muted">{t("detail.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if course}
    <header class="row" style="justify-content: space-between">
        <h1>{course.name}</h1>
        <div class="row">
            <a href={`/courses/${id}/edit`} class="button">{t("detail.edit")}</a>
            <a href={`/courses/${id}/audit`} class="button">{t("detail.audit")}</a>
            <button class="button danger" onclick={handleDelete}>{t("detail.delete")}</button>
        </div>
    </header>

    <section class="surface stack">
        <h2>{t("detail.identity")}</h2>
        <dl class="kv">
            <dt>{t("detail.id")}</dt><dd><code>{course.id}</code></dd>
            <dt>{t("detail.courseCode")}</dt><dd>{course.course_code ?? t("detail.empty")}</dd>
            <dt>{t("detail.status")}</dt><dd>{course.status ?? t("detail.empty")}</dd>
            <dt>{t("detail.educationalLevel")}</dt><dd>{levelLabel(course)}</dd>
            <dt>{t("detail.numberOfCredits")}</dt><dd>{course.number_of_credits ?? t("detail.empty")}</dd>
            <dt>{t("detail.timeRequired")}</dt><dd>{course.time_required ?? t("detail.empty")}</dd>
            <dt>{t("detail.description")}</dt><dd>{course.description ?? t("detail.empty")}</dd>
            <dt>{t("detail.url")}</dt><dd>{#if course.url}<a href={course.url} target="_blank" rel="noopener">{course.url}</a>{:else}{t("detail.empty")}{/if}</dd>
            <dt>{t("detail.license")}</dt><dd>{#if course.license}<a href={course.license} target="_blank" rel="noopener">{course.license}</a>{:else}{t("detail.empty")}{/if}</dd>
            <dt>{t("detail.free")}</dt><dd>{course.is_accessible_for_free === null || course.is_accessible_for_free === undefined ? t("detail.empty") : course.is_accessible_for_free ? t("detail.yes") : t("detail.no")}</dd>
        </dl>
    </section>

    {#if course.identifiers && course.identifiers.length > 0}
        <section class="surface stack">
            <h2>{t("detail.identifiers")}</h2>
            <ul>
                {#each course.identifiers as identifier}
                    <li>
                        <strong>{identifierLabel(identifier)}</strong>
                        <code>{identifier.value}</code>
                        {#if identifier.url}<a href={identifier.url} target="_blank" rel="noopener" class="small">↗</a>{/if}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if course.teaches && course.teaches.length > 0}
        <section class="surface stack">
            <h2>{t("detail.teaches")}</h2>
            <ul>
                {#each course.teaches as item}<li>{item}</li>{/each}
            </ul>
        </section>
    {/if}

    {#if course.keywords && course.keywords.length > 0}
        <section class="surface stack">
            <h2>{t("detail.keywords")}</h2>
            <p>{course.keywords.join(", ")}</p>
        </section>
    {/if}

    {#if course.alternate_names && course.alternate_names.length > 0}
        <section class="surface stack">
            <h2>{t("detail.alternateNames")}</h2>
            <ul>
                {#each course.alternate_names as alt}<li>{alt}</li>{/each}
            </ul>
        </section>
    {/if}

    {#if course.same_as && course.same_as.length > 0}
        <section class="surface stack">
            <h2>{t("detail.sameAs")}</h2>
            <ul>
                {#each course.same_as as href}<li><a {href} target="_blank" rel="noopener">{href}</a></li>{/each}
            </ul>
        </section>
    {/if}

    {#if course.instances && course.instances.length > 0}
        <section class="surface stack">
            <h2>{t("detail.instances")} <span class="muted small">({course.instances.length})</span></h2>
            <ul>
                {#each course.instances as inst}
                    <li>
                        <strong>{instanceLabel(inst)}</strong>
                        {#if inst.status}<span class="muted small">{inst.status}</span>{/if}
                        {#if inst.maximum_attendee_capacity != null}
                            <span class="muted small">{t("detail.capacity")} {inst.enrolled_count ?? 0}/{inst.maximum_attendee_capacity}</span>
                        {/if}
                    </li>
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
