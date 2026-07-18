<!--
  Sign-in page (BFF, per-app magic-link login, PF-T18). Posts to the
  `default` server action, which calls the authentication service
  server-side with a return URL pointing back at THIS app's /verify.
  No token is held in the browser.
-->
<script lang="ts">
  import type { ActionData } from "./$types";
  import { enhance } from "$app/forms";

  let { form }: { form: ActionData } = $props();
</script>

<svelte:head><title>Sign in — Contact Relationship Management</title></svelte:head>

<h1>Sign in</h1>

{#if form?.sent}
  <div class="panel">
    <p>Check your email for a sign-in link.</p>
  </div>
{:else}
  <div class="panel">
    <form class="row" method="POST" use:enhance>
      <label>
        Email
        <input type="email" name="email" required autocomplete="email" />
      </label>
      <button class="primary" type="submit">Send magic link</button>
    </form>
    {#if form?.error}
      <p class="error" role="alert">
        Could not send the sign-in link. Please try again.
      </p>
    {/if}
  </div>
{/if}
