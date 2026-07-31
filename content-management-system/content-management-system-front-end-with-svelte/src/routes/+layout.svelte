<script lang="ts">
  import "../app.css";
  import { LOCALE_LABELS, i18n, isRtl, t } from "$lib/i18n.svelte";
  import { LocalePicker } from "lily-design-system-svelte-locale-picker";
  import { ThemePicker } from "lily-design-system-svelte-theme-picker";

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

  // The app sets `lang`/`dir` itself rather than letting LocalePicker do
  // it (`applyDir={false}` below): the component would mirror the
  // document from its own value, which fights this effect. The family
  // hit this in every front-end; the workaround is to own it here.
  //
  // Note the package names: Lily renamed these helpers `*-select` →
  // `*-picker`. The sibling front-ends still reference the old paths,
  // which no longer exist — copy-adapting one of them today gets an
  // install failure, not a working app.
  $effect(() => {
    document.documentElement.lang = i18n.locale;
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
  <label class="locale">
    <span class="visually-hidden">{t("chrome.language")}</span>
    <LocalePicker
      label={t("chrome.language")}
      locales={[...i18n.locales]}
      localeLabels={LOCALE_LABELS}
      value={i18n.locale}
      applyDir={false}
      onChange={(code) => i18n.set(code)}
    />
  </label>
  <ThemePicker
    label={t("chrome.theme")}
    themesUrl="/assets/themes/"
    themes={THEMES}
    storageKey="mxi.cms.theme"
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
  /* The Lily pickers render a button plus a listbox, not a `<select>`
     — the older front-ends' `:global(select)` rules are left over from
     the `*Select` components these replaced and match nothing. */
  .locale :global(.locale-picker-button),
  nav.top :global(.theme-picker-button) {
    font: inherit;
    padding: 0.15rem 0.3rem;
    max-width: 11rem;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
