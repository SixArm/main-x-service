<!--
  Verify page (BFF, CMS-T25). The magic-link exchange happens entirely
  in the server `load`, which sets the httpOnly session cookie and
  redirects home on success — so this renders ONLY on a
  missing/invalid link.
-->
<script lang="ts">
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();

  const message = $derived(
    data.error === "missingToken"
      ? "This sign-in link is missing its token."
      : "This sign-in link is invalid or has expired.",
  );
</script>

<svelte:head><title>Sign-in link — Content Management System</title></svelte:head>

<h1>Sign-in link</h1>
<div class="panel">
  <p class="error" role="alert">{message}</p>
  <p><a href="/signin">Request a new link</a></p>
</div>
