<!--
  Sign-up page (BFF): create a NEW passwordless account + trigger a magic
  link. Mirrors sign-in, plus an optional name. The form posts to the
  `default` server action (`+page.server.ts`), which calls the
  authentication service server-side; no token is held in the browser.
-->
<script lang="ts">
    import type { ActionData } from "./$types";
    import { enhance } from "$app/forms";
    import { i18n, t } from "$lib/i18n.svelte";

    let { form }: { form: ActionData } = $props();
</script>

<svelte:head><title>{t("signup.title")} — {t("brand")}</title></svelte:head>

<h1>{t("signup.title")}</h1>

{#if form?.sent}
    <p class="banner">{t("signup.sent")}</p>
    <p><a href="/signin">{t("signup.backToSignin")}</a></p>
{:else}
    <form class="stack" method="POST" use:enhance>
        <label>
            {t("signup.email")}
            <input type="email" name="email" required autocomplete="email" />
        </label>
        <label>
            {t("signup.name")} <small>{t("signup.nameOptional")}</small>
            <input type="text" name="name" autocomplete="name" />
        </label>
        <input type="hidden" name="locale" value={i18n.locale} />
        <button class="button" type="submit">{t("signup.submit")}</button>
        {#if form?.error}
            <p class="banner" role="alert">{t("signup.failed")}</p>
        {/if}
    </form>
    <p>
        <small>{t("signup.haveAccount")} <a href="/signin">{t("signup.signin")}</a></small>
    </p>
{/if}
