<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { i18n, isRtl, t, LOCALES, LOCALE_LABELS } from "$lib/i18n.svelte";
  import PickerBar from "@lilydesignsystem/svelte-picker-bar";
  import type { ShareTarget } from "@lilydesignsystem/svelte-share-picker";

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

  // Share destinations for the Lily SharePicker. Lily ships no
  // third-party URLs — each `href` builder is ours. `url`/`title` are
  // supplied by SharePicker at share time (current page URL; the leaf
  // page's title, sourced from `page.data.title` above).
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

  $effect(() => {
    // `lang` must read as BCP 47 (hyphenated); `i18n.locale` uses an
    // underscore for a region subtag (e.g. "en_US") — this must agree
    // with what PickerBar's LocalePicker itself writes via its own
    // `bcp47LocaleTag`, since both write the same attribute.
    document.documentElement.lang = i18n.locale.replace("_", "-");
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
    <PickerBar
      labels={{
        theme: "Theme",
        locale: t("chrome.language"),
        textSize: t("nav.text_size"),
        share: t("nav.share"),
      }}
      themesUrl="/assets/themes/"
      themeProps={{ storageKey: "mxi.crm.theme" }}
      locales={[...LOCALES]}
      localeProps={{
        value: i18n.locale,
        localeLabels: LOCALE_LABELS,
        applyDir: false,
        onChange: (code: string) => i18n.set(code),
      }}
      textSizeProps={{
        storageKey: "mxi.crm.text-size",
      }}
      shareTargets={SHARE_TARGETS}
      shareProps={{
        title: pageTitle,
        copyLabel: t("share.copy_link"),
        copiedLabel: t("share.copied"),
        copyFailedLabel: t("share.copy_failed"),
      }}
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
  .chrome :global(.picker-bar) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }
  .chrome :global(.theme-picker-button),
  .chrome :global(.locale-picker-button),
  .chrome :global(.text-size-picker-button),
  .chrome :global(.share-picker-button) {
    font: inherit;
    padding: 0.15rem 0.3rem;
    max-width: 11rem;
  }
</style>
