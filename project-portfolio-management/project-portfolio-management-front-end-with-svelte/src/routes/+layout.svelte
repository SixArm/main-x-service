<!--
  Root layout — chrome shared by every route.

  Purpose:
    Renders the top navigation bar (brand, nav, session panel) plus the
    routed page.

  $props:
    - children: Snippet — the routed page content (`{@render children()}`).
    - data: LayoutData — `signedIn` resolved server-side from the httpOnly
            session cookie (`+layout.server.ts`).

  Session affordance: per-app magic-link login on this app's own `/signin`;
  sign-out posts to the root page's `signout` action (BFF: revokes the
  session server-side + clears the cookie). The browser never holds a token.
-->
<script lang="ts">
  import "../app.css";
  import { browser } from "$app/environment";
  import { page } from "$app/state";
  import { enhance } from "$app/forms";
  import type { Snippet } from "svelte";
  import type { LayoutData } from "./$types";
  import { i18n, t, isRtl } from "$lib/i18n.svelte";
  import { COLLECTIONS } from "$lib/api/types";
  import { ThemePicker } from "lily-design-system-svelte-theme-picker";
  import { SharePicker, type ShareTarget } from "lily-design-system-svelte-share-picker";
  import { TextSizePicker } from "lily-design-system-svelte-text-size-picker";

  // Lily theme catalogue offered in the theme select (incl.
  // NHS England/Scotland/Wales patient & practitioner themes). Each slug
  // has a Lily stylesheet at `static/assets/themes/<slug>.css` (a symlink
  // to the shared design-system themes) that ThemePicker swaps in.
  const THEMES = [
    "abyss", "acid", "aqua", "autumn", "black", "bumblebee", "business",
    "caramellatte", "cmyk", "coffee", "corporate", "cupcake", "cyberpunk",
    "dark", "dim", "dracula", "emerald", "fantasy", "forest", "garden",
    "halloween", "lemonade", "light", "lofi", "luxury", "night", "nord",
    "pastel", "retro", "silk", "sunset", "synthwave",
    "united-kingdom-national-health-service-england-for-patients",
    "united-kingdom-national-health-service-england-for-practitioners",
    "united-kingdom-national-health-service-scotland-for-patients",
    "united-kingdom-national-health-service-scotland-for-practitioners",
    "united-kingdom-national-health-service-wales-for-patients",
    "united-kingdom-national-health-service-wales-for-practitioners",
    "valentine", "winter", "wireframe"
  ];

    // Human-readable labels for the theme select — the FULL theme name for
    // each slug (DaisyUI names title-cased; the NHS slugs spelled out in full).
    const THEME_LABELS: Record<string, string> = {
        abyss: "Abyss", acid: "Acid", aqua: "Aqua", autumn: "Autumn",
        black: "Black", bumblebee: "Bumblebee", business: "Business",
        caramellatte: "Caramellatte", cmyk: "Cmyk", coffee: "Coffee",
        corporate: "Corporate", cupcake: "Cupcake", cyberpunk: "Cyberpunk",
        dark: "Dark", dim: "Dim", dracula: "Dracula", emerald: "Emerald",
        fantasy: "Fantasy", forest: "Forest", garden: "Garden",
        halloween: "Halloween", lemonade: "Lemonade", light: "Light",
        lofi: "Lofi", luxury: "Luxury", night: "Night", nord: "Nord",
        pastel: "Pastel", retro: "Retro", silk: "Silk", sunset: "Sunset",
        synthwave: "Synthwave", valentine: "Valentine", winter: "Winter",
        wireframe: "Wireframe",
        "united-kingdom-national-health-service-england-for-patients": "United Kingdom National Health Service England for Patients",
        "united-kingdom-national-health-service-england-for-practitioners": "United Kingdom National Health Service England for Practitioners",
        "united-kingdom-national-health-service-scotland-for-patients": "United Kingdom National Health Service Scotland for Patients",
        "united-kingdom-national-health-service-scotland-for-practitioners": "United Kingdom National Health Service Scotland for Practitioners",
        "united-kingdom-national-health-service-wales-for-patients": "United Kingdom National Health Service Wales for Patients",
        "united-kingdom-national-health-service-wales-for-practitioners": "United Kingdom National Health Service Wales for Practitioners",
    };

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

  // `data.signedIn` is resolved server-side from the httpOnly session
  // cookie (`+layout.server.ts`).
  let { children, data }: { children: Snippet; data: LayoutData } = $props();

  // The `page.data.title` convention: each route's own load function
  // (`+page.ts`/`+page.server.ts`) returns a plain `title` string that
  // mirrors what that route's `<svelte:head><title>` renders, so the
  // layout — which does not know which leaf page is active — can read
  // it here for SharePicker without scraping `document.title`. Falls
  // back to the brand name for the rare route that sets none.
  const pageTitle = $derived(page.data?.title ?? t("brand.name"));

  // Hamburger toggle state for the top navigation bar (narrow viewports).
  let menuOpen = $state(false);

  // The i18n store is the single source of truth for the locale; mirror it
  // onto `<html lang>` / `<html dir>` (RTL for ar/ur) whenever it changes.
  // SSR-guarded — `document` is only touched in the browser.
  $effect(() => {
    const locale = i18n.locale;
    if (!browser || typeof document === "undefined") return;
    document.documentElement.lang = locale;
    document.documentElement.dir = isRtl(locale) ? "rtl" : "ltr";
  });

  // Sidebar navigation targets; `key` is an i18n key so labels follow the
  // selected locale. `aria-current` is set per item below.
  const navItems: { href: string; label: string }[] = [
    { href: "/", label: t("brand.name") },
    ...COLLECTIONS.map((collection) => ({
      href: `/${collection}`,
      label: collection,
    })),
    // Record merge (fold a confirmed duplicate plan into a survivor).
    { href: "/plans/merge", label: t("nav.merge") },
    // PPM catalogue views (fully localized 2026-07-18).
    { href: "/dashboard", label: t("ppm.nav.dashboard") },
    { href: "/prioritisation", label: "Prioritisation" },
    { href: "/lifecycle", label: "Lifecycle" },
    { href: "/reviews", label: "Reviews" },
    { href: "/automations", label: "Automations" },
    { href: "/proposals", label: t("ppm.nav.proposals") },
    { href: "/ideas", label: t("ppm.nav.ideas") },
    { href: "/scenarios", label: t("ppm.nav.scenarios") },
    { href: "/objectives", label: t("ppm.nav.objectives") },
    { href: "/gantt", label: t("ppm.nav.gantt") },
    { href: "/engineering", label: t("ppm.nav.engineering") },
    { href: "/calendar", label: t("ppm.nav.calendar") },
    { href: "/executive", label: t("ppm.nav.executive") },
    { href: "/financials", label: t("ppm.nav.financials") },
    { href: "/technology", label: t("ppm.nav.technology") },
    { href: "/board", label: t("ppm.nav.board") },
    { href: "/auditor", label: t("ppm.nav.auditor") },
    { href: "/compliance", label: t("ppm.nav.compliance") },
    { href: "/risk", label: t("ppm.nav.risk") },
    { href: "/security", label: t("ppm.nav.security") },
    { href: "/regulator", label: t("ppm.nav.regulator") },
    { href: "/capacity", label: t("ppm.nav.capacity") },
    { href: "/reports", label: t("ppm.nav.reports") },
  ];

  // Reactive: tracks the server-resolved session presence.
  const signedIn = $derived(data.signedIn);
</script>

<div class="layout">
  <header class="topbar">
    <button
      type="button"
      class="hamburger"
      aria-expanded={menuOpen}
      aria-controls="primary-nav"
      aria-label={t("nav.toggle")}
      onclick={() => (menuOpen = !menuOpen)}
    >
      <span class="hamburger-box" aria-hidden="true"></span>
    </button>
    <a href="/" class="brand">{t("brand.name")}</a>
    <nav id="primary-nav" class="primary-nav" class:open={menuOpen}>
      <ul>
        {#each navItems as item (item.href)}
          <li>
            <a
              href={item.href}
              aria-current={page.url.pathname === item.href ? "page" : undefined}
              onclick={() => (menuOpen = false)}
            >
              {item.label}
            </a>
          </li>
        {/each}
      </ul>

      <div class="chrome">
        <ThemePicker
          label={t("chrome.theme")}
          themesUrl="/assets/themes/"
          themes={THEMES}
          themeLabels={THEME_LABELS}
          storageKey="lily-theme"
        />
        <TextSizePicker
          label={t("nav.text_size")}
          sizes={SIZES}
          sizeLabels={SIZE_LABELS}
          defaultValue="medium"
          storageKey="lily-text-size"
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

      <div class="session">
      <div class="session-title small muted">{t("session.title")}</div>
      {#if signedIn}
        <p class="small" data-testid="session-status">{t("session.tokenAttached")}</p>
        <!-- Sign-out posts to the root page's `signout` action
             (BFF: revokes server-side + clears the cookie). -->
        <form method="POST" action="/?/signout" use:enhance>
          <button class="button danger small" type="submit">
            {t("session.clearToken")}
          </button>
        </form>
      {:else}
        <p class="small" data-testid="session-status">{t("session.noToken")}</p>
        <!-- Per-app magic-link login on this app's own origin. -->
        <a class="button primary small signin" href="/signin">
          {t("session.signIn")}
        </a>
      {/if}
      <p class="small muted">
        {t("session.hint")}
      </p>
    </div>
    </nav>
  </header>
  <main>{@render children()}</main>
</div>

<style>
  .layout {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }
  .topbar {
    position: relative;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 1rem;
    padding: 0.75rem 1.5rem;
    background: var(--mxi-color-surface);
    border-bottom: 1px solid var(--mxi-color-border);
  }
  .brand {
    font-weight: 700;
    color: inherit;
    text-decoration: none;
    white-space: nowrap;
  }
  .hamburger {
    /* Always visible: the primary nav is collapsed behind it at every width. */
    display: block;
    width: 2.5rem;
    height: 2.5rem;
    padding: 0;
    background: transparent;
    border: 1px solid var(--mxi-color-border);
    border-radius: var(--mxi-radius);
    cursor: pointer;
  }
  .hamburger-box,
  .hamburger-box::before,
  .hamburger-box::after {
    display: block;
    width: 1.1rem;
    height: 2px;
    margin: 0 auto;
    background: currentColor;
    content: "";
  }
  .hamburger-box::before { transform: translateY(-5px); }
  .hamburger-box::after { transform: translateY(3px); }
  .primary-nav {
    /* Always collapsed behind the hamburger: hidden by default at every
       width, shown only when the toggle adds `.open`. Rendered as a dropdown
       panel overlaying content (position:absolute) so opening it does not
       reflow the header. */
    display: none;
    position: absolute;
    top: 100%;
    left: 1.5rem;
    z-index: 20;
    flex-direction: column;
    align-items: stretch;
    gap: 0.75rem;
    min-width: 16rem;
    padding: 0.75rem;
    background: var(--mxi-color-surface);
    border: 1px solid var(--mxi-color-border);
    border-radius: var(--mxi-radius);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  }
  .primary-nav.open { display: flex; }
  .primary-nav ul {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin: 0;
    padding: 0;
  }
  .primary-nav a {
    display: block;
    text-decoration: none;
    padding: 0.5rem 0.625rem;
    border-radius: var(--mxi-radius);
    color: inherit;
  }
  .primary-nav a:hover { background: var(--mxi-color-bg); }
  .primary-nav a[aria-current="page"] {
    background: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
    font-weight: 600;
  }
  main {
    width: 100%;
    padding: 1.5rem 2rem;
  }
  .chrome {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.75rem;
  }
  .chrome :global(.theme-picker-button),
  .chrome :global(.text-size-picker-button),
  .chrome :global(.share-picker-button) {
    padding: 0.375rem 0.5rem;
    font-size: 0.875rem;
    color: var(--mxi-color-fg);
    background: var(--mxi-color-bg, transparent);
    border: 1px solid var(--mxi-color-border);
    border-radius: 0.25rem;
    cursor: pointer;
  }
  .session {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--mxi-color-border);
  }
  .session-title {
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .session form {
    margin: 0;
  }
  .session .signin {
    display: block;
    width: 100%;
    text-align: center;
    text-decoration: none;
  }
</style>
