<script lang="ts">
    import { AuthRepository } from "$lib/api/auth";
    import { i18n, t } from "$lib/i18n.svelte";

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
            await repo.requestMagicLink(email, i18n.locale);
            sent = true;
        } catch (err) {
            error = err instanceof Error ? err.message : t("signin.failed");
        } finally {
            submitting = false;
        }
    }
</script>

<svelte:head><title>{t("signin.title")} — {t("brand")}</title></svelte:head>

<h1>{t("signin.title")}</h1>

{#if sent}
    <p class="banner">{t("signin.sent")}</p>
{:else}
    <form class="stack" onsubmit={handleSubmit}>
        <label>
            {t("signin.email")}
            <input type="email" bind:value={email} required autocomplete="email" />
        </label>
        <button class="button" type="submit" disabled={submitting}>
            {submitting ? t("signin.submitting") : t("signin.submit")}
        </button>
        {#if error}
            <p class="banner" role="alert">{error}</p>
        {/if}
    </form>
    <p>
        <small>{t("signin.noAccount")} <a href="/signup">{t("signin.create")}</a></small>
    </p>
{/if}
