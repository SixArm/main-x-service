<!--
  Edit route (`/[pid]/edit`): loads a record, seeds OrganizationForm with
  it, and PUTs the result.

  $state:
    - org:     Organization | null — loaded record (seed for the form).
    - loading: boolean             — true until the fetch settles.
    - error:   string | null       — fetch failure (inline banner).

  The form renders only once `org` is loaded so it seeds from real data.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import OrganizationForm from "$lib/components/OrganizationForm.svelte";
    import { OrganizationRepository } from "$lib/api/organizations";
    import { t } from "$lib/i18n.svelte";
    import type { Organization } from "$lib/api/types";

    const repo = OrganizationRepository.withFetch();
    // Route param; `?? ""` satisfies strict typing (param is always set here).
    const pid = page.params.pid ?? "";

    let org = $state<Organization | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);

    // Load the record to edit once on mount; not-found surfaces as `error`.
    onMount(async () => {
        try {
            org = await repo.get(pid);
        } catch (err) {
            error = err instanceof Error ? err.message : t("edit.notFound");
        } finally {
            loading = false;
        }
    });

    /** Persist the edited record, then return to its detail page. */
    async function handleSubmit(updated: Organization) {
        await repo.update(pid, updated);
        await goto(`/${pid}`);
    }
</script>

<svelte:head><title>{t("edit.title")}: {org?.name ?? t("edit.organizationFallback")} — Main X</title></svelte:head>

<h1>{t("edit.title")}</h1>

{#if loading}
    <p>{t("edit.loading")}</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if org}
    <OrganizationForm initial={org} submitLabel={t("edit.saveChanges")} onsubmit={handleSubmit} />
{/if}
