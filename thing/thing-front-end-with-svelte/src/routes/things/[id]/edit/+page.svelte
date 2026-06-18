<!--
  +page.svelte (/things/[id]/edit) — edit an existing Thing.

  Purpose: loads the Thing by route id and seeds ThingForm with it; on submit
  saves via update() and navigates back to the detail page.

  $state:
    - thing: loaded record used as the form's initial value.
    - error / loading: request status.

  Reactive notes: `id` is $derived from page.params; record loads in onMount.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import ThingForm from "$lib/components/ThingForm.svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import type { Thing } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    const repo = ThingRepository.withFetch();
    let thing = $state<Thing | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    const id = $derived(page.params.id as string);

    onMount(async () => {
        try {
            thing = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Persist the edited Thing, then return to its detail view.
    async function handleSubmit(value: Thing) {
        await repo.update(id, value);
        goto(`/things/${id}`);
    }
</script>

<svelte:head><title>Edit thing · {id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("edit.title")}</h1>
    <a href={`/things/${id}`} class="button">{t("edit.cancel")}</a>
</header>

{#if loading}
    <p class="muted">{t("edit.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if thing}
    <section class="surface stack">
        <ThingForm initial={thing} submitLabel={t("edit.submitLabel")} onsubmit={handleSubmit} />
    </section>
{/if}
