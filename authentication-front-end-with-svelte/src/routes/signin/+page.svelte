<script lang="ts">
    import { AuthRepository } from "$lib/api/auth";

    const repo = AuthRepository.withFetch();

    let email = $state("");
    let submitting = $state(false);
    let sent = $state(false);
    let error = $state<string | null>(null);

    async function handleSubmit(event: SubmitEvent) {
        event.preventDefault();
        error = null;
        submitting = true;
        try {
            await repo.requestMagicLink(email);
            sent = true;
        } catch (err) {
            error = err instanceof Error ? err.message : "Request failed";
        } finally {
            submitting = false;
        }
    }
</script>

<svelte:head><title>Sign in — Main X Auth</title></svelte:head>

<h1>Sign in</h1>

{#if sent}
    <p class="banner">
        If that email has an account, a magic link is on its way. In development
        the link is printed to the auth service console — open it to sign in.
    </p>
{:else}
    <form class="stack" onsubmit={handleSubmit}>
        <label>
            Email
            <input type="email" bind:value={email} required autocomplete="email" />
        </label>
        <button class="button" type="submit" disabled={submitting}>
            {submitting ? "Sending…" : "Email me a magic link"}
        </button>
        {#if error}
            <p class="banner" role="alert">{error}</p>
        {/if}
    </form>
    <p><small>No account yet? <a href="/signup">Create one</a></small></p>
{/if}
