<!--
  Verify page (BFF). The magic-link exchange happens entirely in the
  server `load`, which sets the httpOnly session cookie and redirects home
  on success — so this renders ONLY on a missing/invalid link. The browser
  never handles an access token.
-->
<script lang="ts">
    import type { PageData } from "./$types";

    let { data }: { data: PageData } = $props();

    const message = $derived(
        data.error === "missingToken"
            ? "This sign-in link is missing its token."
            : data.error === "serviceUnavailable"
              ? "We could not reach the sign-in service. Please try again in a moment."
              : "This sign-in link is invalid or has expired.",
    );
</script>

<svelte:head><title>Sign-in link</title></svelte:head>

<h1>Sign-in link</h1>
<p class="banner" role="alert">{message}</p>
<p><a class="button" href="/signin">Request a new link</a></p>
