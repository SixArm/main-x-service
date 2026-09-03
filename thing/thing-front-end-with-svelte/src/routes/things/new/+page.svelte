<!--
  +page.svelte (/things/new) — create a new Thing.

  Purpose: renders ThingForm; on submit creates the Thing and navigates to
  its detail page. If the service reports duplicates (HTTP 409), shows the
  candidate list and blocks creation until the user resubmits.

  $state:
    - duplicates: candidate MatchResult[] surfaced from a 409 conflict.

  Reactive notes: handleSubmit re-throws so ThingForm's submit-error banner
  reflects the duplicate warning; clearing/repopulating `duplicates` drives
  the MatchResultsList below the form.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import ThingForm from "$lib/components/ThingForm.svelte";
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MatchResult, Thing } from "$lib/api/types.js";
    import { t, translate } from "$lib/i18n.svelte.js";

    const repo = ThingRepository.withFetch();
    let duplicates = $state<MatchResult[]>([]);

    // A new Thing starts with just an empty name (the only required field).
    const blank: Thing = { name: "" };

    async function handleSubmit(value: Thing) {
        duplicates = [];
        try {
            const created = await repo.create(value);
            if (created.id) goto(`/things/${created.id}`);
        } catch (err) {
            // 409 with array details = duplicate candidates: surface them and
            // re-throw a friendly message so the form shows the warning banner.
            if (
                err instanceof ApiError &&
                err.isConflict &&
                Array.isArray(err.details)
            ) {
                duplicates = err.details as MatchResult[];
                throw new Error(
                    translate("new.duplicatesDetected").replace(
                        "{count}",
                        String(duplicates.length),
                    ),
                );
            }
            // Any other error bubbles up to the form's submit-error handling.
            throw err;
        }
    }
</script>

<svelte:head><title>New thing · Thing Service</title></svelte:head>

<header><h1>{t("new.title")}</h1></header>

<section class="surface stack">
    <ThingForm
        initial={blank}
        submitLabel={t("new.submitLabel")}
        onsubmit={handleSubmit}
    />
</section>

{#if duplicates.length > 0}
    <MatchResultsList
        results={duplicates}
        title={t("new.possibleDuplicates")}
    />
{/if}
