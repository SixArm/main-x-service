<!--
  Home / account dashboard (BFF).

  The signed-in user is resolved server-side from the httpOnly session
  cookie (layout `load` → /token exchange → /me), so there is no client
  token, no GET /me on mount, and no localStorage. Sign-out posts to the
  `signout` server action, which revokes the session and clears the cookie.
-->
<script lang="ts">
    import type { PageData } from "./$types";
    import { enhance } from "$app/forms";
    import { t } from "$lib/i18n.svelte";

    // `user` comes from the layout server load (merged into page data).
    let { data }: { data: PageData } = $props();
</script>

<svelte:head><title>{t("brand")}</title></svelte:head>

<h1>{t("account.title")}</h1>

{#if data.user}
    <div class="surface stack">
        <div><strong>{t("account.name")}</strong> {data.user.name}</div>
        <div><strong>{t("account.email")}</strong> {data.user.email}</div>
        <div><strong>{t("account.id")}</strong> <code>{data.user.pid}</code></div>
        <p>
            <small><a href="/admin/attributes">Manage user attributes (admin)</a></small>
        </p>
        <form method="POST" action="?/signout" use:enhance>
            <button class="button" type="submit">{t("account.signout")}</button>
        </form>
    </div>
{:else}
    <p>{t("account.notSignedIn")}</p>
    <p>
        <a class="button" href="/signin">{t("account.signinPrompt.signin")}</a>
        {t("account.signinPrompt.or")}
        <a href="/signup">{t("account.signinPrompt.create")}</a>.
    </p>
{/if}
