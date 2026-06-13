<script lang="ts">
    import { onMount } from "svelte";
    import { page } from "$app/state";
    import { AuthRepository } from "$lib/api/auth";
    import { RETURN_TO_ALLOWLIST } from "$lib/config";
    import { parseAllowlist, persistReturnTo } from "$lib/auth/return-to";
    import { i18n, t } from "$lib/i18n.svelte";

    const repo = AuthRepository.withFetch();

    // Park an allowlisted `?return_to=` across the magic-link email
    // round-trip; `/verify` reads it back. The emailed link does not
    // carry it, so we persist it client-side here. Not allowlisted ⇒
    // ignored (never persisted, never handed the token later).
    onMount(() => {
        persistReturnTo(
            page.url,
            parseAllowlist(RETURN_TO_ALLOWLIST),
            window.location.origin,
        );
    });

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
            await repo.signup(email, name.trim() ? name.trim() : undefined, i18n.locale);
            sent = true;
        } catch (err) {
            error = err instanceof Error ? err.message : t("signup.failed");
        } finally {
            submitting = false;
        }
    }
</script>

<svelte:head><title>{t("signup.title")} — {t("brand")}</title></svelte:head>

<h1>{t("signup.title")}</h1>

{#if sent}
    <p class="banner">{t("signup.sent")}</p>
    <p><a href="/signin">{t("signup.backToSignin")}</a></p>
{:else}
    <form class="stack" onsubmit={handleSubmit}>
        <label>
            {t("signup.email")}
            <input type="email" bind:value={email} required autocomplete="email" />
        </label>
        <label>
            {t("signup.name")} <small>{t("signup.nameOptional")}</small>
            <input type="text" bind:value={name} autocomplete="name" />
        </label>
        <button class="button" type="submit" disabled={submitting}>
            {submitting ? t("signup.submitting") : t("signup.submit")}
        </button>
        {#if error}
            <p class="banner" role="alert">{error}</p>
        {/if}
    </form>
    <p>
        <small>{t("signup.haveAccount")} <a href="/signin">{t("signup.signin")}</a></small>
    </p>
{/if}
