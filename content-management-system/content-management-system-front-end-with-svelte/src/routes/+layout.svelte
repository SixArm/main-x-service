<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { i18n, isRtl, t, LOCALES, LOCALE_LABELS } from "$lib/i18n.svelte";
  import PickerBar from "@lilydesignsystem/svelte-picker-bar";
  import type { ShareTarget } from "@lilydesignsystem/svelte-share-picker";

  let { children } = $props();

  // Lily theme catalogue offered in the theme picker (DaisyUI-style
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
  const pageTitle = $derived(page.data?.title ?? t("brand.name"));

  // Reflect the active locale onto <html> so the document language and
  // writing direction track the UI: `lang` for the locale (hyphenated
  // per BCP 47 — `i18n.locale` uses an underscore for a region subtag,
  // e.g. "en_US", but `lang` must read "en-US"; this must agree with
  // what PickerBar's LocalePicker itself writes via its own
  // `bcp47LocaleTag`, since both write the same attribute), `dir` = rtl
  // for Arabic/Urdu else ltr. The effect below is the single owner of
  // `<html lang>`/`dir` — PickerBar's LocalePicker is wired with
  // `applyDir={false}` below so it does not also try to own `dir`.
  //
  // Note the package names: Lily renamed these helpers `*-select` →
  // `*-picker`. The sibling front-ends still reference the old paths,
  // which no longer exist — copy-adapting one of them today gets an
  // install failure, not a working app.
  $effect(() => {
    document.documentElement.lang = i18n.locale.replace("_", "-");
    document.documentElement.dir = isRtl(i18n.locale) ? "rtl" : "ltr";
  });
</script>

<nav class="top">
  <a class="brand" href="/">{t("brand.name")}</a>
  <a href="/">{t("nav.dashboard")}</a>
  <a href="/entries">{t("nav.entries")}</a>
  <a href="/assets">{t("nav.assets")}</a>
  <a href="/workflow">{t("nav.workflow")}</a>
  <a href="/translations">{t("nav.translations")}</a>
  <a href="/insights">{t("nav.insights")}</a>
  <a href="/settings">{t("nav.settings")}</a>
  <span class="spacer"></span>
  <PickerBar
    labels={{
      theme: t("chrome.theme"),
      locale: t("chrome.language"),
      textSize: t("nav.text_size"),
      share: t("nav.share"),
    }}
    themesUrl="/assets/themes/"
    themes={THEMES}
    themeProps={{ storageKey: "mxi.cms.theme" }}
    locales={[...LOCALES]}
    localeProps={{
      value: i18n.locale,
      localeLabels: LOCALE_LABELS,
      applyDir: false,
      onChange: (code: string) => i18n.set(code),
    }}
    sizes={SIZES}
    textSizeProps={{
      sizeLabels: SIZE_LABELS,
      defaultValue: "medium",
      storageKey: "lily-text-size",
    }}
    shareTargets={SHARE_TARGETS}
    shareProps={{
      title: pageTitle,
      copyLabel: t("share.copy_link"),
      copiedLabel: t("share.copied"),
      copyFailedLabel: t("share.copy_failed"),
    }}
  />
  <a href="/signin">{t("nav.signin")}</a>
</nav>

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
  /* The Lily pickers render a button plus a listbox, not a `<select>`
     — the older front-ends' `:global(select)` rules are left over from
     the `*Select` components these replaced and match nothing. */
  nav.top :global(.theme-picker-button),
  nav.top :global(.locale-picker-button),
  nav.top :global(.text-size-picker-button),
  nav.top :global(.share-picker-button) {
    font: inherit;
    padding: 0.15rem 0.3rem;
    max-width: 11rem;
  }
</style>
