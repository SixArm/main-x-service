<script lang="ts">
    import { AuthRepository } from "$lib/api/auth";

    const repo = AuthRepository.withFetch();

    let email = $state("");
    let name = $state("");
    let submitting = $state(false);
    let sent = $state(false);
    let error = $state<string | null>(null);

    async function handleSubmit(event: SubmitEvent) {
        event.preventDefault();
        error = null;
        submitting = true;
        try {
            await repo.signup(email, name.trim() ? name.trim() : undefined);
            sent = true;
        } catch (err) {
            error = err instanceof Error ? err.message : "Sign up failed";
        } finally {
            submitting = false;
        }
    }
</script>

<svelte:head><title>Sign up — Main X Auth</title></svelte:head>

<h1>Create account</h1>

{#if sent}
    <p class="banner">
        If that email is valid, a magic link is on its way. In development the
        link is printed to the auth service console — open it to finish signing
        in.
    </p>
    <p><a href="/signin">Back to sign in</a></p>
{:else}
    <form class="stack" onsubmit={handleSubmit}>
        <label>
            Email
            <input type="email" bind:value={email} required autocomplete="email" />
        </label>
        <label>
            Name <small>(optional)</small>
            <input type="text" bind:value={name} autocomplete="name" />
        </label>
        <button class="button" type="submit" disabled={submitting}>
            {submitting ? "Sending…" : "Send magic link"}
        </button>
        {#if error}
            <p class="banner" role="alert">{error}</p>
        {/if}
    </form>
    <p><small>Already have an account? <a href="/signin">Sign in</a></small></p>
{/if}
