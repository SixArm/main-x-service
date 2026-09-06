<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { i18n, isRtl, t } from "$lib/i18n.svelte";
  import { ThemePicker } from "lily-design-system-svelte-theme-picker";
  import { SharePicker, type ShareTarget } from "lily-design-system-svelte-share-picker";
  import { TextSizePicker } from "lily-design-system-svelte-text-size-picker";

  let { children } = $props();

  // The `page.data.title` convention: each route's own load function
  // (`+page.ts`/`+page.server.ts`) returns a plain `title` string that
  // mirrors what that route's `<svelte:head><title>` renders, so the
  // layout — which does not know which leaf page is active — can read
  // it here for SharePicker without scraping `document.title`. Falls
  // back to the brand name for the rare route that sets none. Optional
  // chaining is required — `page.data` is undefined in a bare test
  // render with no `data` prop.
  const pageTitle = $derived(page.data?.title ?? t("brand.name"));

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
  // page's title, sourced from `page.data.title` above).
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

  // Lily theme catalogue offered in the theme select (DaisyUI-style
  // slugs plus government/NHS design-system themes). Each slug has a
  // stylesheet at `static/assets/themes/<slug>.css` (a symlink to the
  // shared design-system themes) that ThemePicker swaps in; labels are
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


  $effect(() => {
    document.documentElement.lang = i18n.locale;
    document.documentElement.dir = isRtl(i18n.locale) ? "rtl" : "ltr";
  });
</script>

<nav class="top">
  <a class="brand" href="/">{t("brand.name")}</a>
  <a href="/contacts">{t("nav.contacts")}</a>
  <a href="/accounts">{t("nav.accounts")}</a>
  <a href="/leads">{t("nav.leads")}</a>
  <a href="/deals">{t("nav.deals")}</a>
  <a href="/campaigns">{t("nav.campaigns")}</a>
  <a href="/tickets">{t("nav.tickets")}</a>
  <a href="/articles">{t("nav.articles")}</a>
  <a href="/followups">{t("nav.followups")}</a>
  <a href="/executive">{t("nav.executive")}</a>
  <a href="/dpo">{t("nav.dpo")}</a>
  <a href="/engagement">{t("nav.engagement")}</a>
  <a href="/partners">{t("nav.partners")}</a>
  <span class="spacer"></span>
  <div class="chrome">
    <ThemePicker
      label="Theme"
      themesUrl="/assets/themes/"
      themes={THEMES}
      storageKey="mxi.crm.theme"
    />
    <TextSizePicker
      label={t("nav.text_size")}
      sizes={SIZES}
      sizeLabels={SIZE_LABELS}
      defaultValue="medium"
      storageKey="mxi.crm.text-size"
    />
    <SharePicker
      label={t("nav.share")}
      title={pageTitle}
      targets={SHARE_TARGETS}
      copyLabel={t("share.copy_link")}
      copiedLabel={t("share.copied")}
      copyFailedLabel={t("share.copy_failed")}
    />
  </div>
  <a href="/signin">{t("nav.signin")}</a>
</nav>

<main>
  {@render children()}
</main>

<style>
  .spacer {
    flex: 1;
  }
  .chrome {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .chrome :global(.theme-picker-button),
  .chrome :global(.text-size-picker-button),
  .chrome :global(.share-picker-button) {
    font: inherit;
    padding: 0.15rem 0.3rem;
    max-width: 11rem;
  }
</style>
