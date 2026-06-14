<!--
  Sign-in page: request a magic link for an EXISTING account.

  Flow: on mount, park any allowlisted `?return_to=` so /verify can hand
  the token to the originating operator app after sign-in. Submitting the
  form posts the email to POST /api/auth/magic-link with the current UI
  locale (so the email language matches), then shows a "sent" confirmation
  (deliberately not revealing whether the account exists).

  State:
  - `email` ($state) — bound to the email input;
  - `submitting` ($state) — disables the button while the request is in flight;
  - `sent` ($state) — switches the view to the confirmation banner;
  - `error` ($state) — request error message.

  Events:
  - `handleSubmit` — form submit handler.
-->
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
    let submitting = $state(false);
    let sent = $state(false);
    let error = $state<string | null>(null);

    // Request a magic link for the entered email in the current UI locale.
    async function handleSubmit(event: SubmitEvent) {
        event.preventDefault();
        error = null;
        submitting = true;
        try {
            // locale ⇒ the magic-link email is sent in the chosen language.
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
