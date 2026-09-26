<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import PickerBar from "@lilydesignsystem/svelte-picker-bar";
  import type { ShareTarget } from "@lilydesignsystem/svelte-share-picker";

  let { children } = $props();

  // Locale ids offered by the Lily LocalePicker (bundled into PickerBar).
  // This project has no i18n store — the picker is self-contained: it sets
  // `lang`/`dir` on <html> itself, with no value/onChange wiring needed.
  const LOCALES = [
    "en", "en_US", "cy", "es", "fr", "de", "ar", "ru", "hi", "zh", "bn", "pt", "id", "ur",
  ] as const;
  const LOCALE_LABELS: Record<string, string> = {
    en: "English",
    en_US: "English (United States)",
    cy: "Cymraeg",
    es: "Español",
    fr: "Français",
    de: "Deutsch",
    ar: "العربية",
    ru: "Русский",
    hi: "हिन्दी",
    zh: "中文",
    bn: "বাংলা",
    pt: "Português",
    id: "Bahasa Indonesia",
    ur: "اردو",
  };

  // Kiosk routes are chrome-less (wall touchscreens).
  let kiosk = $derived(page.url.pathname.endsWith("/kiosk"));

  $effect(() => {
    document.body.classList.toggle("kiosk", kiosk);
  });

  // Share destinations for the Lily SharePicker. Lily ships no
  // third-party URLs — each `href` builder is ours. `url`/`title` are
  // supplied by SharePicker at share time (current page URL; the leaf
  // page's title, sourced from `page.data.title` below — the
  // `page.data.title` convention, set per-route by each route's load
  // function so it stays in sync with that page's own <svelte:head>
  // <title> without SharePicker having to read the DOM).
  const SHARE_TARGETS: ShareTarget[] = [
    {
      id: "email",
      label: "Email",
      href: (url, title) =>
        `mailto:?subject=${encodeURIComponent(title)}&body=${encodeURIComponent(url)}`,
      newTab: false,
    },
    {
      id: "linkedin",
      label: "LinkedIn",
      href: (url) =>
        `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(url)}`,
    },
    {
      id: "reddit",
      label: "Reddit",
      href: (url, title) =>
        `https://www.reddit.com/submit?url=${encodeURIComponent(url)}&title=${encodeURIComponent(title)}`,
    },
    {
      id: "bluesky",
      label: "Bluesky",
      href: (url, title) =>
        `https://bsky.app/intent/compose?text=${encodeURIComponent(`${title} ${url}`)}`,
    },
    {
      id: "mastodon",
      label: "Mastodon",
      href: (url, title) =>
        `https://mastodonshare.com/?text=${encodeURIComponent(title)}&url=${encodeURIComponent(url)}`,
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
    <PickerBar
      labels={{
        theme: "Theme",
        locale: "Language",
        textSize: "Text size",
        share: "Share",
      }}
      themesUrl="/assets/themes/"
      themeProps={{ storageKey: "mxi.patient-flow.theme" }}
      locales={[...LOCALES]}
      localeProps={{ localeLabels: LOCALE_LABELS }}
      textSizeProps={{
        storageKey: "mxi.patient-flow.text-size",
      }}
      shareTargets={SHARE_TARGETS}
      shareProps={{
        title: pageTitle,
        copyLabel: "Copy link",
        copiedLabel: "Link copied",
        copyFailedLabel: "Could not copy — copy it from the address bar",
      }}
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
  nav.top :global(.picker-bar) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }
  nav.top :global(.theme-picker-button),
  nav.top :global(.locale-picker-button),
  nav.top :global(.text-size-picker-button),
  nav.top :global(.share-picker-button) {
    font: inherit;
    padding: 0.15rem 0.3rem;
    max-width: 11rem;
  }
</style>
