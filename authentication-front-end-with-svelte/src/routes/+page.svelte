<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { AuthRepository } from "$lib/api/auth";
    import { ApiError } from "$lib/api/client";
    import { session } from "$lib/auth/session.svelte";

    const repo = AuthRepository.withFetch();

    let loading = $state(true);
    let error = $state<string | null>(null);

    onMount(async () => {
        if (!session.isAuthenticated || !session.token) {
            loading = false;
            return;
        }
        try {
            const user = await repo.me(session.token);
            session.setUser(user);
        } catch (err) {
            // Token expired or revoked — drop it and show the signed-out view.
            if (err instanceof ApiError && err.isUnauthorized) {
                session.clear();
            } else {
                error = err instanceof Error ? err.message : "Failed to load profile";
            }
        } finally {
            loading = false;
        }
    });

    async function handleSignout() {
        if (session.token) {
            try {
                await repo.signout(session.token);
            } catch {
                // Best-effort: clear locally regardless.
            }
        }
        session.clear();
        await goto("/signin");
    }
</script>

<svelte:head><title>Main X Auth</title></svelte:head>

<h1>Account</h1>

{#if loading}
    <p>Loading…</p>
{:else if session.isAuthenticated && session.user}
    <div class="surface stack">
        <div><strong>Name:</strong> {session.user.name}</div>
        <div><strong>Email:</strong> {session.user.email}</div>
        <div><strong>ID:</strong> <code>{session.user.pid}</code></div>
        <button class="button" onclick={handleSignout}>Sign out</button>
    </div>
{:else}
    <p>You are not signed in.</p>
    <p>
        <a class="button" href="/signin">Sign in</a>
        or
        <a href="/signup">create an account</a>.
    </p>
{/if}

{#if error}
    <p class="banner" role="alert">{error}</p>
{/if}
