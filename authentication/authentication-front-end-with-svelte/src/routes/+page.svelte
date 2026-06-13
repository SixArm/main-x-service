<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { AuthRepository } from "$lib/api/auth";
    import { ApiError } from "$lib/api/client";
    import { session } from "$lib/auth/session.svelte";
    import { t } from "$lib/i18n.svelte";

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
                error = err instanceof Error ? err.message : t("account.loadFailed");
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

<svelte:head><title>{t("brand")}</title></svelte:head>

<h1>{t("account.title")}</h1>

{#if loading}
    <p>{t("account.loading")}</p>
{:else if session.isAuthenticated && session.user}
    <div class="surface stack">
        <div><strong>{t("account.name")}</strong> {session.user.name}</div>
        <div><strong>{t("account.email")}</strong> {session.user.email}</div>
        <div><strong>{t("account.id")}</strong> <code>{session.user.pid}</code></div>
        <button class="button" onclick={handleSignout}>{t("account.signout")}</button>
    </div>
{:else}
    <p>{t("account.notSignedIn")}</p>
    <p>
        <a class="button" href="/signin">{t("account.signinPrompt.signin")}</a>
        {t("account.signinPrompt.or")}
        <a href="/signup">{t("account.signinPrompt.create")}</a>.
    </p>
{/if}

{#if error}
    <p class="banner" role="alert">{error}</p>
{/if}
