<script lang="ts">
  // Root layout — the app shell wrapping every route.
  //
  // Purpose: render the top navigation bar (brand, nav, session controls) and the
  // active route via the `children` snippet.
  //
  // Props ($props): `children: Snippet` — the active page, rendered with
  // `{@render children()}`; `data: LayoutData` — carries `signedIn`,
  // resolved server-side from the httpOnly session cookie
  // (`+layout.server.ts`), so the chrome shows signed-in state without the
  // browser ever holding a token. See `agents/share/authentication-sessions.md`.
  //
  // Events: nav links are plain anchors; sign-out posts to the root page's
  // `signout` action (BFF); sign-in links to this app's own `/signin`.
  import "../app.css";
  import { browser } from '$app/env';
  import { page } from "$app/state";
  import { enhance } from "$app/forms";
  import type { Snippet } from "svelte";
  import type { LayoutData } from "./$types";
  import { i18n, t, isRtl, LOCALES, LOCALE_LABELS } from "#lib/i18n.svelte.js";
  import PickerBar from "@lilydesignsystem/svelte-picker-bar";
  import type { ShareTarget } from "@lilydesignsystem/svelte-share-picker";

  // Share destinations for the Lily SharePicker. Lily ships no
  // third-party URLs — each `href` builder is ours. `url`/`title` are
  // supplied by SharePicker at share time (current page URL; the leaf
  // page's title, sourced from `page.data.title` below — the
  // `page.data.title` convention, set per-route by each route's load
  // function so it stays in sync with that page's own <svelte:head>
  // <title> without SharePicker having to read the DOM).
  const SHARE_TARGETS: ShareTarget[] = $derived([
    {
      id: "email",
      label: t("share.email"),
      href: (url, title) =>
        `mailto:?subject=${encodeURIComponent(title)}&body=${encodeURIComponent(url)}`,
      newTab: false,
    },
    {
      id: "linkedin",
      label: t("share.linkedin"),
      href: (url) =>
        `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(url)}`,
    },
    {
      id: "reddit",
      label: t("share.reddit"),
      href: (url, title) =>
        `https://www.reddit.com/submit?url=${encodeURIComponent(url)}&title=${encodeURIComponent(title)}`,
    },
    {
      id: "bluesky",
      label: t("share.bluesky"),
      href: (url, title) =>
        `https://bsky.app/intent/compose?text=${encodeURIComponent(`${title} ${url}`)}`,
    },
    {
      id: "mastodon",
      label: t("share.mastodon"),
      href: (url, title) =>
        `https://mastodonshare.com/?text=${encodeURIComponent(title)}&url=${encodeURIComponent(url)}`,
    },
  ]);

  // `data.signedIn` is resolved server-side from the httpOnly session
  // cookie (`+layout.server.ts`).
  let { children, data }: { children: Snippet; data: LayoutData } = $props();

  // The `page.data.title` convention: each route's own load function
  // (`+page.ts`/`+page.server.ts`) returns a plain `title` string that
  // mirrors what that route's `<svelte:head><title>` renders, so the
  // layout — which does not know which leaf page is active — can read
  // it here for SharePicker without scraping `document.title`. Falls
  // back to the brand name for the rare route that sets none.
  const pageTitle = $derived(page.data?.title ?? t("brand.full"));

  // Hamburger toggle state for the top navigation bar (narrow viewports).
  let menuOpen = $state(false);

  // The i18n store is the single source of truth for the locale: this
  // effect mirrors it onto `<html lang>` / `<html dir>` (RTL for ar-001).
  // SSR-guarded — only touches the document in the browser. `lang` is
  // hyphenated per BCP 47 (locale codes are already hyphenated, e.g.
  // "en-001" or "zh-cn"; this must agree with what PickerBar's
  // LocalePicker itself writes via its own `bcp47LocaleTag`, since both
  // write the same attribute).
  $effect(() => {
    if (!browser) return;
    document.documentElement.lang = i18n.locale.replace("_", "-");
    document.documentElement.dir = isRtl(i18n.locale) ? "rtl" : "ltr";
  });

  // Nav items reference i18n keys; labels are resolved reactively in markup.
  const navItems = [
    { href: "/", key: "nav.pathways" as const },
    { href: "/new", key: "nav.newCarePathway" as const },
    { href: "/insights", key: "nav.insights" as const },
    { href: "/board", key: "nav.board" as const },
    { href: "/gantt", key: "nav.gantt" as const },
    { href: "/sequence", key: "nav.sequence" as const },
    { href: "/time", key: "nav.time" as const },
    { href: "/bulk", key: "nav.bulk" as const },
    { href: "/review-queue", key: "nav.reviewQueue" as const },
    { href: "/tour", key: "nav.tour" as const },
  ];
</script>

<div class="layout">
  <header class="topbar">
    <button
      type="button"
      class="hamburger"
      aria-expanded={menuOpen}
      aria-controls="primary-nav"
      aria-label={t("nav.toggle")}
      onclick={() => menuOpen = !menuOpen}
    ><span class="hamburger-box" aria-hidden="true"></span></button>

    <a href="/" class="brand">{t("brand.full")}</a>
    <nav id="primary-nav" class="primary-nav" class:open={menuOpen}>
      <ul>
        {#each navItems as item (item.href)}
          <li>
            <a
              href={item.href}
              aria-current={page.url.pathname === item.href ? "page" : undefined}
              onclick={() => menuOpen = false}
            >{t(item.key)}</a>
          </li>
        {/each}
      </ul>
    </nav>
    <div class="header-end">
      {#if data.signedIn}
        <!-- Sign-out posts to the root page's `signout` action (BFF:
             revokes server-side + clears the cookie). -->
        <form method="POST" action="/?/signout" use:enhance>
          <button
            type="submit"
            class="session-button"
            data-testid="session-status"
            >{t("auth.signout")}</button
          >
        </form>
      {:else}
        <!-- Per-app magic-link login on this app's own origin. -->
        <a class="session-button signin" href="/signin">{t("auth.signin")}</a>
      {/if}
      <PickerBar
        labels={{
          theme: t("chrome.theme"),
          locale: t("chrome.language"),
          textSize: t("chrome.textSize"),
          share: t("chrome.share"),
        }}
        themesUrl="/assets/themes/"
        themeProps={{
          storageKey: "lily-theme",
          // Without a default no theme stylesheet loads until the user
          // picks one, leaving the pickers (which Lily's theme CSS styles)
          // unstyled on first visit. Follow the OS light/dark preference,
          // else fall back to "light".
          detectFromSystem: true,
          defaultValue: "light",
        }}
        locales={[...LOCALES]}
        localeProps={{
          value: i18n.locale,
          localeLabels: LOCALE_LABELS,
          applyDir: false,
          onChange: (code: string) => i18n.set(code),
        }}
        textSizeProps={{ storageKey: "lily-text-size" }}
        shareTargets={SHARE_TARGETS}
        shareProps={{
          title: pageTitle,
          copyLabel: t("share.copyLink"),
          copiedLabel: t("share.linkCopied"),
          copyFailedLabel: t("share.copyFailed"),
        }}
      />
    </div>
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
    border-bottom: 1px solid var(--mxi-color-border, #ddd);
  }
  .brand {
    font-weight: 700;
    color: inherit;
    text-decoration: none;
    white-space: nowrap;
  }
  .hamburger {
    /* Always visible at every width: the primary nav is always collapsed
             behind this toggle (not a responsive show-full-nav-on-desktop
             pattern). */
    display: block;
    width: 2.5rem;
    height: 2.5rem;
    padding: 0;
    background: transparent;
    border: 1px solid var(--mxi-color-border, #ddd);
    border-radius: 6px;
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
  .hamburger-box::before {
    transform: translateY(-5px);
  }
  .hamburger-box::after {
    transform: translateY(3px);
  }
  .primary-nav {
    /* Always collapsed behind the hamburger: hidden by default at every
             width, shown only when the toggle adds `.open`. Rendered as a
             dropdown panel overlaying content (position:absolute) so opening it
             does not reflow the header. */
    display: none;
    position: absolute;
    top: 100%;
    inset-inline-start: 1.5rem;
    z-index: 20;
    flex-direction: column;
    align-items: stretch;
    gap: 0.75rem;
    min-width: 16rem;
    padding: 0.75rem;
    background: var(--mxi-color-surface, #fff);
    border: 1px solid var(--mxi-color-border, #ddd);
    border-radius: var(--mxi-radius, 6px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  }
  .primary-nav.open {
    display: flex;
  }
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
    padding: 0.4rem 0.5rem;
    border-radius: 6px;
    color: inherit;
  }
  .primary-nav a[aria-current="page"] {
    background: var(--mxi-color-primary, #2563eb);
    color: var(--mxi-color-primary-fg, #fff);
    font-weight: 600;
  }
  main {
    width: 100%;
    padding: 1.5rem 2rem;
  }
  .header-end {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-inline-start: auto;
  }
  .header-end form {
    margin: 0;
  }
  .session-button {
    display: inline-flex;
    align-items: center;
    height: 2.5rem;
    padding: 0 0.875rem;
    border: 1px solid var(--mxi-color-border);
    border-radius: var(--mxi-radius);
    background: transparent;
    color: var(--mxi-color-fg);
    font: inherit;
    font-size: 0.875rem;
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
    text-decoration: none;
    cursor: pointer;
  }
  .session-button:hover {
    background: var(--mxi-color-bg);
    text-decoration: none;
  }
  .session-button.signin {
    border-color: var(--mxi-color-primary);
    background: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
  }
  .session-button.signin:hover {
    background: var(--mxi-color-primary);
    filter: brightness(1.1);
  }

  /* Lily PickerBar. The Lily theme stylesheet (loaded by ThemePicker, see
       `defaultValue` below) styles the picker buttons and listboxes; this
       only places them. Each listbox is anchored to the header (`.topbar`
       is `position: relative`) rather than to its own button, so it drops
       down under the header's end edge, is as wide as its longest label
       (the theme names are long), never pushes the page down, and cannot
       run off either side of a narrow screen. */
  .header-end :global(.picker-bar) {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }
  .header-end :global(.theme-picker),
  .header-end :global(.locale-picker),
  .header-end :global(.text-size-picker),
  .header-end :global(.share-picker) {
    position: static;
    display: inline-flex;
    align-items: center;
  }
  /* "Copied" feedback: a live region Lily renders inside the share
       picker's root. In flow it made that root taller than the other
       three and knocked its button out of line with them, so it is lifted
       out of flow and shown as a small pill under the header instead. */
  .header-end :global(.share-picker-status) {
    position: absolute;
    top: 100%;
    inset-inline-end: 1.5rem;
    z-index: 40;
    margin: 0.25rem 0 0;
    padding: 0.25rem 0.625rem;
    border-radius: var(--mxi-radius);
    background: var(--mxi-color-surface);
    border: 1px solid var(--mxi-color-border);
    font-size: 0.8125rem;
  }
  .header-end :global(.share-picker-status:empty) {
    display: none;
  }
  .header-end :global(.theme-picker-button),
  .header-end :global(.locale-picker-button),
  .header-end :global(.text-size-picker-button),
  .header-end :global(.share-picker-button) {
    box-sizing: border-box;
    width: 2.5rem;
    height: 2.5rem;
    margin: 0;
    vertical-align: middle;
  }
  .header-end :global(.theme-picker-list),
  .header-end :global(.locale-picker-list),
  .header-end :global(.text-size-picker-list),
  .header-end :global(.share-picker-list) {
    top: 100%;
    inset-inline-start: auto;
    inset-inline-end: 1.5rem;
    z-index: 40;
    box-sizing: border-box;
    min-width: 0;
    width: max-content;
    max-width: calc(100vw - 2rem);
  }
  .header-end :global(.theme-picker-option),
  .header-end :global(.locale-picker-option),
  .header-end :global(.text-size-picker-option) {
    white-space: nowrap;
  }
  @media (max-width: 40rem) {
    .topbar {
      padding-inline: 0.75rem;
      gap: 0.5rem;
    }
    .header-end {
      gap: 0.375rem;
    }
    .header-end :global(.theme-picker-option),
    .header-end :global(.locale-picker-option),
    .header-end :global(.text-size-picker-option) {
      white-space: normal;
    }
    .header-end :global(.share-picker-status) {
      inset-inline-end: 0.75rem;
    }
    .header-end :global(.theme-picker-list),
    .header-end :global(.locale-picker-list),
    .header-end :global(.text-size-picker-list),
    .header-end :global(.share-picker-list) {
      inset-inline-end: 0.75rem;
    }
  }
</style>
