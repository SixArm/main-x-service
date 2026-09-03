<!--
  Sign-in page (BFF, per-app magic-link login). Posts to the `default`
  server action, which calls the authentication service server-side with a
  return URL pointing back at THIS app's /verify. No token is held in the
  browser. (Copy uses plain English; full i18n for the signin/verify pages
  is a follow-up — the entity apps had no login UI before the BFF.)
-->
<script lang="ts">
    import type { ActionData } from "./$types";
    import { enhance } from "$app/forms";
    import { i18n } from "$lib/i18n.svelte.js";

    let { form }: { form: ActionData } = $props();
</script>

<svelte:head><title>Sign in</title></svelte:head>

<h1>Sign in</h1>

{#if form?.sent}
    <p class="banner">Check your email for a sign-in link.</p>
{:else}
    <form class="stack" method="POST" use:enhance>
        <label>
            Email
            <input type="email" name="email" required autocomplete="email" />
        </label>
        <input type="hidden" name="locale" value={i18n.locale} />
        <button class="button" type="submit">Send magic link</button>
        {#if form?.error}
            <p class="banner" role="alert">
                Could not send the sign-in link. Please try again.
            </p>
        {/if}
    </form>
{/if}
