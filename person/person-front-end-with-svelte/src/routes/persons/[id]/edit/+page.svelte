<!--
  Edit person (/persons/[id]/edit) — load a record, edit it via PersonForm,
  and save back.

  Loads the existing record on mount and seeds PersonForm with it; on submit
  it updates the record and navigates to the detail page. Any update error
  propagates into the form's submitError.

  State:
    - person — the loaded record used as the form's initial value.
    - error / loading — load lifecycle.
    - id ($derived) — the person id from the route params.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import PersonForm from "$lib/components/PersonForm.svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { Person } from "$lib/api/types.js";

    const repo = PersonRepository.withFetch();
    let person = $state<Person | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    const id = $derived(page.params.id as string);

    // Load the record to edit on mount (SSR disabled).
    onMount(async () => {
        try {
            person = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Persist the edited record, then go to the detail view. Thrown errors
    // bubble into PersonForm's submitError.
    async function handleSubmit(value: Person) {
        await repo.update(id, value);
        goto(`/persons/${id}`);
    }
</script>

<svelte:head><title>{t("edit.head.title.prefix")}{id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("edit.title")}</h1>
    <a href={`/persons/${id}`} class="button">{t("edit.cancel")}</a>
</header>

{#if loading}
    <p class="muted">{t("edit.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if person}
    <section class="surface stack">
        <PersonForm
            initial={person}
            submitLabel={t("edit.save")}
            onsubmit={handleSubmit}
        />
    </section>
{/if}
