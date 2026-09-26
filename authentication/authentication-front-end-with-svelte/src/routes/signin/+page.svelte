<!--
  Sign-in page (BFF): request a magic link for an EXISTING account.

  The form posts to the `default` server action (`+page.server.ts`), which
  calls the authentication service server-side. No token is ever held in
  the browser. A hidden `locale` field carries the UI language so the
  magic-link email matches. The confirmation is deliberately generic (it
  does not reveal whether the account exists).

  The "Sign in with SSO" link (EV-2, `agents/share/authentication-sessions.md`
  §7a) is a plain <a>, not a form action — it is a BROWSER NAVIGATION to
  this app's own `/signin/sso` route, which 303-redirects onward to the
  auth service's `/api/auth/oidc/login`. A `fetch`-based BFF call cannot
  do this hop: the browser itself must visit the identity provider and
  come back. Shown only when `PUBLIC_OIDC_SIGNIN_ENABLED` is set (see
  `.env.example`) — magic link stays the default, federation is opt-in.
-->
<script lang="ts">
    import type { ActionData } from "./$types";
    import { enhance } from "$app/forms";
    import { i18n, t } from "$lib/i18n.svelte";
    import { env } from "$env/dynamic/public";

    let { form }: { form: ActionData } = $props();
    const ssoEnabled = env.PUBLIC_OIDC_SIGNIN_ENABLED === "true";
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
        {#if form?.error === "rate-limited"}
            <p class="banner" role="alert">{t("account.rateLimited")}</p>
        {:else if form?.error}
            <p class="banner" role="alert">{t("signin.failed")}</p>
        {/if}
    </form>
    {#if ssoEnabled}
        <p>
            <a class="button" href="/signin/sso">{t("signin.sso")}</a>
        </p>
    {/if}
    <p>
        <small>{t("signin.noAccount")} <a href="/signup">{t("signin.create")}</a></small>
    </p>
{/if}
