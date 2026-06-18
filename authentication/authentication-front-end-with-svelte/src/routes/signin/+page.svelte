<!--
  Sign-in page (BFF): request a magic link for an EXISTING account.

  The form posts to the `default` server action (`+page.server.ts`), which
  calls the authentication service server-side. No token is ever held in
  the browser. A hidden `locale` field carries the UI language so the
  magic-link email matches. The confirmation is deliberately generic (it
  does not reveal whether the account exists).
-->
<script lang="ts">
    import type { ActionData } from "./$types";
    import { enhance } from "$app/forms";
    import { i18n, t } from "$lib/i18n.svelte";

    let { form }: { form: ActionData } = $props();
</script>

<svelte:head><title>{t("signin.title")} — {t("brand")}</title></svelte:head>

<h1>{t("signin.title")}</h1>

{#if form?.sent}
    <p class="banner">{t("signin.sent")}</p>
{:else}
    <form class="stack" method="POST" use:enhance>
        <label>
            {t("signin.email")}
            <input type="email" name="email" required autocomplete="email" />
        </label>
        <input type="hidden" name="locale" value={i18n.locale} />
        <button class="button" type="submit">{t("signin.submit")}</button>
        {#if form?.error}
            <p class="banner" role="alert">{t("signin.failed")}</p>
        {/if}
    </form>
    <p>
        <small>{t("signin.noAccount")} <a href="/signup">{t("signin.create")}</a></small>
    </p>
{/if}
