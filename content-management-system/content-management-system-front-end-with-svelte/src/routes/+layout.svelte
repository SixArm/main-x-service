<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { enhance } from "$app/forms";
  import { browser } from '$app/env';
  import { i18n, isRtl, t, LOCALES, LOCALE_LABELS } from "#lib/i18n.svelte.js";
  import PickerBar from "@lilydesignsystem/svelte-picker-bar";
  import type { ShareTarget } from "@lilydesignsystem/svelte-share-picker";

  let { children, data } = $props();

  // `data.signedIn` is resolved server-side from the httpOnly session
  // cookie (`+layout.server.ts`).
  const signedIn = $derived(data.signedIn);

  // Hamburger toggle for the primary navigation.
  let menuOpen = $state(false);

  const navItems = [
    { href: "/", key: "nav.dashboard" },
    { href: "/entries", key: "nav.entries" },
    { href: "/assets", key: "nav.assets" },
    { href: "/workflow", key: "nav.workflow" },
    { href: "/translations", key: "nav.translations" },
    { href: "/insights", key: "nav.insights" },
    { href: "/settings", key: "nav.settings" },
    { href: "/tour", key: "nav.tour" },
  ] as const;

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
    if (!browser) return;
    document.documentElement.lang = i18n.locale;
    document.documentElement.dir = isRtl(i18n.locale) ? "rtl" : "ltr";
  });
</script>

<div class="layout">
  <header class="topbar">
    {#if signedIn}
      <button
        type="button"
        class="hamburger"
        aria-expanded={menuOpen}
        aria-controls="primary-nav"
        aria-label={t("nav.toggle")}
        onclick={() => menuOpen = !menuOpen}
      ><span class="hamburger-box" aria-hidden="true"></span></button>
    {/if}
    <a href="/" class="brand">{t("brand.name")}</a>
    {#if signedIn}
      <nav id="primary-nav" class="primary-nav" class:open={menuOpen}>
        <ul>
          {#each navItems as item}
            <li>
              <a
                href={item.href}
                aria-current={page.url.pathname === item.href ? "page" : null}
                onclick={() => menuOpen = false}
              >{t(item.key)}</a>
            </li>
          {/each}
        </ul>
      </nav>
    {/if}
    <div class="header-end">
      {#if signedIn}
        <!-- Sign-out posts to the BFF's /signout (revokes server-side and
             clears the cookie). -->
        <form method="POST" action="/signout" use:enhance>
          <button type="submit" class="session-button"
            >{t("auth.signout")}</button
          >
        </form>
      {:else}
        <a class="session-button signin" href="/signin">{t("auth.signin")}</a>
      {/if}
      <PickerBar
        labels={{
          theme: t("chrome.theme"),
          locale: t("chrome.language"),
          textSize: t("nav.text_size"),
          share: t("nav.share"),
        }}
        themesUrl="/assets/themes/"
        themeProps={{
          storageKey: "lily-theme",
          // Without a default no theme stylesheet loads until the user
          // picks one, leaving the pickers (which Lily's theme CSS
          // styles) unstyled on first visit. Follow the OS light/dark
          // preference, else fall back to "light".
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
        textSizeProps={{
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
    </div>
  </header>
  <main>
    {@render children()}
  </main>
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
        background: var(--panel);
        border-bottom: 1px solid var(--line);
    }
    .brand {
        font-size: 1.125rem;
        font-weight: 600;
        color: var(--ink);
        white-space: nowrap;
    }
    .brand:hover {
        text-decoration: none;
    }
    .hamburger {
        /* Always visible: the primary nav is collapsed behind this toggle at
           every viewport width (not a responsive show-full-nav-on-desktop
           pattern). */
        display: block;
        width: 2.5rem;
        height: 2.5rem;
        padding: 0;
        background: transparent;
        border: 1px solid var(--line);
        border-radius: var(--radius);
        cursor: pointer;
    }
    .hamburger-box,
    .hamburger-box::before,
    .hamburger-box::after {
        display: block;
        width: 1.1rem;
        height: 2px;
        margin: 0 auto;
        background: var(--ink);
        content: "";
    }
    .hamburger-box::before {
        transform: translateY(-5px);
    }
    .hamburger-box::after {
        transform: translateY(3px);
    }
    .primary-nav {
        /* Collapsed by default at every width; the hamburger toggle adds
           `.open` to reveal it as a dropdown panel below the top bar. */
    display: none;
    position: absolute;
    top: 100%;
    inset-inline-start: 1.5rem;
    z-index: 20;
    flex-direction: column;
    align-items: stretch;
    gap: 0.5rem;
    min-width: 12rem;
    padding: 0.75rem;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--radius);
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
    padding: 0.5rem 0.625rem;
    border-radius: var(--radius);
    color: var(--ink);
  }
  .primary-nav a:hover {
    background: var(--bg);
    text-decoration: none;
  }
  .primary-nav a[aria-current="page"] {
    background: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
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
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: 0.875rem;
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
    text-decoration: none;
    cursor: pointer;
  }
  .session-button:hover {
    background: var(--bg);
    text-decoration: none;
  }
  .session-button.signin {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-fg);
  }
  .session-button.signin:hover {
    background: var(--accent);
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
        border-radius: var(--radius);
        background: var(--panel);
        border: 1px solid var(--line);
        font-size: 0.8125rem;
    }
    .header-end :global(.share-picker-status:empty) {
        display: none;
    }
    .header-end :global(.theme-picker-button),
    .header-end :global(.locale-picker-button),
    .header-end :global(.text-size-picker-button),
    .header-end :global(.share-picker-button),
  .header-end :global(.picker-bar button[aria-haspopup]) {
        box-sizing: border-box;
        width: 2.5rem;
        height: 2.5rem;
        margin: 0;
        vertical-align: middle;
    }
    .header-end :global(.theme-picker-list),
    .header-end :global(.locale-picker-list),
    .header-end :global(.text-size-picker-list),
    .header-end :global(.share-picker-list),
  .header-end :global(.picker-bar [role="listbox"]) {
        top: 100%;
        inset-inline-start: auto;
        inset-inline-end: 1.5rem;
        z-index: 40;
        box-sizing: border-box;
        min-width: 0;
        width: max-content;
        max-width: calc(100vw - 2rem);
    }
    /* The published Lily pickers render an `.icon-button` and a plain
     `[role="listbox"]` rather than the `*-picker-button` / `*-picker-list`
     classes, and leave the list in flow: place it explicitly. */
  .header-end :global(.picker-bar [role="listbox"]) {
    position: absolute;
    overflow-y: auto;
    max-height: 70vh;
    margin: 0.25rem 0 0;
    padding: 0.25rem;
    list-style: none;
    background: var(--panel);
    color: var(--ink);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  }
  .header-end :global(.picker-bar [role="listbox"][hidden]) {
    display: none;
  }
  .header-end :global(.picker-bar [role="option"]) {
    padding: 0.375rem 0.625rem;
    border-radius: var(--radius);
    cursor: pointer;
    white-space: nowrap;
  }
  .header-end :global(.picker-bar [role="option"]:hover),
  .header-end :global(.picker-bar [role="option"][aria-selected="true"]) {
    background: var(--bg);
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
    .header-end :global(.share-picker-list),
    .header-end :global(.picker-bar [role="listbox"]) {
      inset-inline-end: 0.75rem;
    }
  }
</style>
