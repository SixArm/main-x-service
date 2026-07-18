<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";

  let { children } = $props();

  // Kiosk routes are chrome-less (wall touchscreens).
  let kiosk = $derived(page.url.pathname.endsWith("/kiosk"));

  $effect(() => {
    document.body.classList.toggle("kiosk", kiosk);
  });
</script>

{#if !kiosk}
  <nav class="top">
    <a class="brand" href="/">Patient Flow</a>
    <a href="/at-a-glance">At a glance</a>
    <a href="/bed-requests">Bed requests</a>
    <a href="/locate">Locate</a>
    <a href="/audits">Audits</a>
    <a href="/signin">Sign in</a>
  </nav>
{/if}

<main>
  {@render children()}
</main>
