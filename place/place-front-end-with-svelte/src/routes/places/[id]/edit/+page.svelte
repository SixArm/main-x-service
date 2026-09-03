<!--
  Edit place (route "/places/[id]/edit") — loads the record, hands it to
  PlaceForm as the initial value, and PUTs the result on submit, then
  returns to the detail page.

  Local $state:
    - place / error / loading — fetched record and request status.
  Derived:
    - id — the route's place id.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import PlaceForm from "$lib/components/PlaceForm.svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { Place } from "$lib/api/types.js";

    const repo = PlaceRepository.withFetch();
    let place = $state<Place | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    const id = $derived(page.params.id as string);

    onMount(async () => {
        try {
            place = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // PlaceForm.onsubmit: persist the edit, then navigate to the detail view.
    // Thrown errors bubble back into the form's submitError banner.
    async function handleSubmit(value: Place) {
        await repo.update(id, value);
        goto(`/places/${id}`);
    }
</script>

<svelte:head><title>Edit place · {id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("edit.title")}</h1>
    <a href={`/places/${id}`} class="button">{t("edit.cancel")}</a>
</header>

{#if loading}
    <p class="muted">{t("edit.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if place}
    <section class="surface stack">
        <PlaceForm
            initial={place}
            submitLabel={t("edit.saveChanges")}
            onsubmit={handleSubmit}
        />
    </section>
{/if}
