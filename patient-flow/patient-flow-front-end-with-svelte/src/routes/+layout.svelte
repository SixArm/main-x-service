<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { ThemePicker } from "lily-design-system-svelte-theme-picker";
  import { SharePicker, type ShareTarget } from "lily-design-system-svelte-share-picker";
  import { TextSizePicker } from "lily-design-system-svelte-text-size-picker";

  let { children } = $props();

  // Kiosk routes are chrome-less (wall touchscreens).
  let kiosk = $derived(page.url.pathname.endsWith("/kiosk"));

  $effect(() => {
    document.body.classList.toggle("kiosk", kiosk);
  });

  // Lily theme catalogue offered in the theme select (DaisyUI-style
  // slugs plus government/NHS design-system themes — the NHS ones are
  // the natural fit here). Each slug has a stylesheet at
  // `static/assets/themes/<slug>.css` (a symlink to the shared
  // design-system themes) that ThemePicker swaps in; labels are
  // title-cased from the slug by the component.
  const THEMES = [
    "abyss", "acid", "adobe-spectrum", "aqua", "autumn", "black",
    "bumblebee", "business", "caramellatte", "cmyk", "coffee",
    "corporate", "cupcake", "cyberpunk", "dark", "dim", "dracula",
    "emerald", "fantasy", "forest", "garden", "halloween", "lemonade",
    "light", "lofi", "luxury", "mozilla-protocol", "night", "nord",
    "pastel", "retro", "silk", "sunset", "synthwave",
    "united-kingdom-government-digital-service",
    "united-kingdom-national-health-service-england-for-patients",
    "united-kingdom-national-health-service-england-for-practitioners",
    "united-kingdom-national-health-service-scotland-for-patients",
    "united-kingdom-national-health-service-scotland-for-practitioners",
    "united-kingdom-national-health-service-wales-for-patients",
    "united-kingdom-national-health-service-wales-for-practitioners",
    "united-states-web-design-system", "valentine", "winter", "wireframe",
  ];

  // Text sizes offered by the Lily TextSizePicker. Applied as
  // `data-text-size` on <html> (attribute-based, mirroring ThemePicker's
  // `data-theme`); see app.css for the corresponding font-size scale.
  const SIZES = ["small", "medium", "large", "x-large"];
  const SIZE_LABELS: Record<string, string> = {
    small: "Small",
    medium: "Medium",
    large: "Large",
    "x-large": "Extra large",
  };

  // Share destinations for the Lily SharePicker. Lily ships no
  // third-party URLs — each `href` builder is ours. `url`/`title` are
  // supplied by SharePicker at share time (current page URL; the leaf
  // page's title, sourced from `page.data.title` below — the
  // `page.data.title` convention, set per-route by each route's load
  // function so it stays in sync with that page's own <svelte:head>
  // <title> without SharePicker having to read the DOM).
  const SHARE_TARGETS: ShareTarget[] = [
    {
      id: "linkedin",
      label: "LinkedIn",
      href: (url) =>
        `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(url)}`,
    },
    {
      id: "mastodon",
      label: "Mastodon",
      href: (url, title) =>
        `https://mastodon.social/share?text=${encodeURIComponent(`${title} ${url}`)}`,
    },
    {
      id: "bluesky",
      label: "Bluesky",
      href: (url, title) =>
        `https://bsky.app/intent/compose?text=${encodeURIComponent(`${title} ${url}`)}`,
    },
    {
      id: "reddit",
      label: "Reddit",
      href: (url, title) =>
        `https://www.reddit.com/submit?url=${encodeURIComponent(url)}&title=${encodeURIComponent(title)}`,
    },
  ];

  // The `page.data.title` convention: each route's own load function
  // (`+page.ts`/`+page.server.ts`) returns a plain `title` string that
  // mirrors what that route's `<svelte:head><title>` renders, so the
  // layout — which does not know which leaf page is active — can read
  // it here for SharePicker without scraping `document.title`. Falls
  // back to the brand name for the rare route that sets none.
  const pageTitle = $derived(page.data?.title ?? "Patient Flow");
</script>

{#if !kiosk}
  <nav class="top">
    <a class="brand" href="/">Patient Flow</a>
    <a href="/wards">Wards</a>
    <a href="/at-a-glance">At a glance</a>
    <a href="/bed-requests">Bed requests</a>
    <a href="/edd">EDD</a>
    <a href="/locate">Locate</a>
    <a href="/audits">Audits</a>
    <span class="spacer"></span>
    <ThemePicker
      label="Theme"
      themesUrl="/assets/themes/"
      themes={THEMES}
      storageKey="mxi.patient-flow.theme"
    />
    <TextSizePicker
      label="Text size"
      sizes={SIZES}
      sizeLabels={SIZE_LABELS}
      defaultValue="medium"
      storageKey="mxi.patient-flow.text-size"
    />
    <SharePicker
      label="Share"
      title={pageTitle}
      targets={SHARE_TARGETS}
      copyLabel="Copy link"
      copiedLabel="Link copied"
      copyFailedLabel="Could not copy — copy it from the address bar"
    />
    <a href="/signin">Sign in</a>
  </nav>
{/if}

<main>
  {@render children()}
</main>

<style>
  .spacer {
    flex: 1;
  }
  nav.top :global(.theme-picker-button),
  nav.top :global(.text-size-picker-button),
  nav.top :global(.share-picker-button) {
    font: inherit;
    padding: 0.15rem 0.3rem;
    max-width: 11rem;
  }
</style>
