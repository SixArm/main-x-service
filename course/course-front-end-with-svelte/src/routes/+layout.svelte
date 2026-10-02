<script lang="ts">
    import "../app.css";
    import { page } from "$app/state";
    import { enhance } from "$app/forms";
    import type { Snippet } from "svelte";
    import type { LayoutData } from "./$types";
    import PickerBar from "@lilydesignsystem/svelte-picker-bar";
    import type { ShareTarget } from "@lilydesignsystem/svelte-share-picker";
    import { browser } from "$app/env";
    import {
        i18n,
        t,
        isRtl,
        LOCALES,
        LOCALE_LABELS,
        type StringKey,
    } from "#lib/i18n.svelte.js";

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

    // Lily headless example — uncomment after `pnpm install` resolves the
    // file: dependency to use Lily's accessibility-primitive Button:
    // import Button from "@lilydesignsystem/svelte-headless/src/lib/components/Button/Button.svelte";

    // `data.signedIn` is resolved server-side from the httpOnly session
    // cookie (`+layout.server.ts`).
    let { children, data }: { children: Snippet; data: LayoutData } = $props();
    const signedIn = $derived(data.signedIn);

    // The `page.data.title` convention: each route's own load function
    // (`+page.ts`/`+page.server.ts`) returns a plain `title` string that
    // mirrors what that route's `<svelte:head><title>` renders, so the
    // layout — which does not know which leaf page is active — can read
    // it here for SharePicker without scraping `document.title`. Falls
    // back to the brand name for the rare route that sets none.
    const pageTitle = $derived(page.data?.title ?? t("brand.name"));

    // Hamburger toggle state for the top navigation bar (narrow viewports).
    let menuOpen = $state(false);

    // Reflect the active locale onto <html> so the document language and
    // writing direction track the UI. Locale codes are already BCP 47
    // (e.g. "en-001", "zh-cn"), which is also what PickerBar's LocalePicker
    // writes, so both agree. `dir` = rtl for Arabic, else ltr.
    $effect(() => {
        if (!browser) return;
        document.documentElement.lang = i18n.locale;
        document.documentElement.dir = isRtl(i18n.locale) ? "rtl" : "ltr";
    });

    // Primary navigation (top bar): href ⇒ active route, label key ⇒ link text.
    const navItems: { href: string; labelKey: StringKey }[] = [
        { href: "/", labelKey: "nav.dashboard" },
        { href: "/courses", labelKey: "nav.courses" },
        { href: "/courses/new", labelKey: "nav.newCourse" },
        { href: "/courses/match", labelKey: "nav.matchCheck" },
        { href: "/courses/merge", labelKey: "nav.merge" },
        { href: "/calendar", labelKey: "nav.calendar" },
        { href: "/board", labelKey: "nav.board" },
        { href: "/tour", labelKey: "nav.tour" },
    ];
</script>

<!--
  Root layout — the app shell wrapping every route: a top navigation bar with the
  brand, primary nav, and the Lily PickerBar — theme/locale/text-size/share,
  plus the routed page content rendered via `children`. Imports global app.css.

  $props:
    - children: Snippet — the active route's rendered output.

  Reactive bits: nav links compare against `page.url.pathname` to set
  `aria-current="page"` on the active item.
-->

<div class="layout">
    <header class="topbar">
        <button
            type="button"
            class="hamburger"
            aria-expanded={menuOpen}
            aria-controls="primary-nav"
            aria-label={t("nav.toggle")}
            onclick={() => (menuOpen = !menuOpen)}
            ><span class="hamburger-box" aria-hidden="true"></span></button
        >

        <a href="/" class="brand">
            {t("brand.name")}
            <span class="muted small tagline">{t("brand.tagline")}</span>
        </a>

        <nav id="primary-nav" class="primary-nav" class:open={menuOpen}>
            <ul>
                {#each navItems as item}
                    <li>
                        <a
                            href={item.href}
                            aria-current={page.url.pathname === item.href
                                ? "page"
                                : null}
                            onclick={() => (menuOpen = false)}
                            >{t(item.labelKey)}</a
                        >
                    </li>
                {/each}
            </ul>
        </nav>
        <div class="header-end">
            {#if signedIn}
                <!-- Sign-out posts to the root page's `signout` action
                     (BFF: revokes server-side + clears the cookie). -->
                <form method="POST" action="/?/signout" use:enhance>
                    <button type="submit" class="session-button"
                        >{t("auth.signout")}</button
                    >
                </form>
            {:else}
                <!-- Per-app magic-link login on this app's own origin. -->
                <a class="session-button signin" href="/signin"
                    >{t("auth.signin")}</a
                >
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
                    // Without a default no theme stylesheet loads until the
                    // user picks one, leaving the pickers (which Lily's theme
                    // CSS styles) unstyled on first visit. Follow the OS
                    // light/dark preference, else fall back to "light".
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
        background: var(--mxi-color-surface);
        border-bottom: 1px solid var(--mxi-color-border);
    }
    .brand {
        font-size: 1.125rem;
        font-weight: 600;
        color: var(--mxi-color-fg);
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
        background: var(--mxi-color-fg);
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
        background: var(--mxi-color-surface);
        border: 1px solid var(--mxi-color-border);
        border-radius: var(--mxi-radius);
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
        border-radius: var(--mxi-radius);
        color: var(--mxi-color-fg);
    }
    .primary-nav a:hover {
        background: var(--mxi-color-bg);
        text-decoration: none;
    }
    .primary-nav a[aria-current="page"] {
        background: var(--mxi-color-primary);
        color: var(--mxi-color-primary-fg);
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
        .tagline {
            display: none;
        }
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
    main {
        width: 100%;
        padding: 1.5rem;
    }
</style>
