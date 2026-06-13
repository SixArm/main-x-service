<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import OrganizationForm from "$lib/components/OrganizationForm.svelte";
    import { OrganizationRepository } from "$lib/api/organizations";
    import type { Organization } from "$lib/api/types";

    const repo = OrganizationRepository.withFetch();
    const pid = page.params.pid ?? "";

    let org = $state<Organization | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);

    onMount(async () => {
        try {
            org = await repo.get(pid);
        } catch (err) {
            error = err instanceof Error ? err.message : "Not found";
        } finally {
            loading = false;
        }
    });

    async function handleSubmit(updated: Organization) {
        await repo.update(pid, updated);
        await goto(`/${pid}`);
    }
</script>

<svelte:head><title>Edit {org?.name ?? "organization"} — Main X</title></svelte:head>

<h1>Edit organization</h1>

{#if loading}
    <p>Loading…</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if org}
    <OrganizationForm initial={org} submitLabel="Save changes" onsubmit={handleSubmit} />
{/if}
