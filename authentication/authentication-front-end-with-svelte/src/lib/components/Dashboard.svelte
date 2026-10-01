<!--
  Dashboard — the signed-in account view (BFF).

  The signed-in user is resolved server-side from the httpOnly session
  cookie (layout `load` → /token exchange → /me), so there is no client
  token, no GET /me on mount, and no localStorage. Sign-out posts to the
  root page's `signout` server action, which revokes the session and
  clears the cookie.

  Props:
  - `user`: the signed-in user from the layout server load.
-->
<script lang="ts">
    import { enhance } from "$app/forms";
    import { t } from "#lib/i18n.svelte.js";

    let {
        user,
    }: { user: { name: string; email: string; pid: string } } = $props();
</script>

<svelte:head><title>{t("brand")}</title></svelte:head>

<h1>{t("account.title")}</h1>

<div class="surface stack">
    <div><strong>{t("account.name")}</strong> {user.name}</div>
    <div><strong>{t("account.email")}</strong> {user.email}</div>
    <div><strong>{t("account.id")}</strong> <code>{user.pid}</code></div>
    <p>
        <small><a href="/admin/attributes">Manage user attributes (admin)</a></small>
    </p>
    <form method="POST" action="?/signout" use:enhance>
        <button class="button" type="submit">{t("account.signout")}</button>
    </form>
</div>
