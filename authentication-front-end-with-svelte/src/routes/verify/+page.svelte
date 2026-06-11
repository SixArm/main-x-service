<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { AuthRepository } from "$lib/api/auth";
    import { session } from "$lib/auth/session.svelte";

    const repo = AuthRepository.withFetch();

    let status = $state<"working" | "error">("working");
    let error = $state<string | null>(null);

    onMount(async () => {
        const token = page.url.searchParams.get("token");
        if (!token) {
            status = "error";
            error = "This link is missing its token.";
            return;
        }
        try {
            const login = await repo.verify(token);
            session.start(login);
            await goto("/");
        } catch (err) {
            status = "error";
            error = err instanceof Error ? err.message : "This link is invalid or expired.";
        }
    });
</script>

<svelte:head><title>Verifying — Main X Auth</title></svelte:head>

{#if status === "working"}
    <h1>Signing you in…</h1>
    <p>Verifying your magic link.</p>
{:else}
    <h1>Could not sign you in</h1>
    <p class="banner" role="alert">{error}</p>
    <p><a class="button" href="/signin">Request a new link</a></p>
{/if}
