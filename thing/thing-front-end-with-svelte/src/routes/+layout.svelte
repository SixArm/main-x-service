<!--
  +layout.svelte — root application shell wrapping every route.

  Purpose: renders the persistent sidebar (brand, primary navigation, theme
  picker, text-size picker, share picker) and a <main> region into which the
  active page is rendered via the `children` snippet.

  $props:
    - children (Snippet): the active route's content.
    - data (LayoutData): `signedIn` resolved server-side from the httpOnly
      session cookie (`+layout.server.ts`).

  Reactive notes: nav items compare against `page.url.pathname` to set
  aria-current="page" on the active link.

  SHARE_TARGETS feeds the share picker. Static config array, not reactive
  state. Theme and text-size choices are Lily's own defaults (see
  `<PickerBar>` below) rather than an app-specific list.
-->
<script lang="ts">
    import "../app.css";
    import { page } from "$app/state";
    import { enhance } from "$app/forms";
    import type { Snippet } from "svelte";
    import type { LayoutData } from "./$types";
    import PickerBar from "@lilydesignsystem/svelte-picker-bar";
    import type { ShareTarget } from "@lilydesignsystem/svelte-share-picker";

    import {
        i18n,
        t,
        isRtl,
        LOCALES,
        LOCALE_LABELS,
    } from "$lib/i18n.svelte.js";

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

    // Lily headless example — uncomment after `pnpm install` resolves the
    // file: dependency to use Lily's accessibility-primitive Button:
    // import Button from "@lilydesignsystem/svelte-headless/src/lib/components/Button/Button.svelte";

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

    // Primary top-bar navigation (href + i18n key for the visible label).
    const navItems = [
        { href: "/", key: "nav.dashboard" as const },
        { href: "/things", key: "nav.things" as const },
        { href: "/things/new", key: "nav.newThing" as const },
        { href: "/things/match", key: "nav.matchCheck" as const },
        { href: "/things/merge", key: "nav.merge" as const },
        { href: "/review", key: "nav.review" as const },
    ];

    // Reflect the active locale onto <html lang> for assistive tech and
    // correct typography, and onto <html dir> so Arabic / Urdu render
    // right-to-left. Guarded for SSR (no document off the browser).
    $effect(() => {
        if (typeof document !== "undefined") {
            document.documentElement.lang = i18n.locale.replace("_", "-");
            document.documentElement.dir = isRtl(i18n.locale) ? "rtl" : "ltr";
        }
    });
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
        <a href="/" class="brand"
            >{t("brand.name")}
            <span class="muted small">{t("brand.tagline")}</span></a
        >
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
                        >
                            {t(item.key)}
                        </a>
                    </li>
                {/each}
            </ul>
            <div class="chrome">
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
            <section class="session" aria-label="Session">
                <div class="session-title">Session</div>
                {#if data.signedIn}
                    <p class="session-status">Signed in</p>
                    <!-- Sign-out posts to the root page's `signout` action
                         (BFF: revokes server-side + clears the cookie). -->
                    <form method="POST" action="/?/signout" use:enhance>
                        <button type="submit">Sign out</button>
                    </form>
                {:else}
                    <!-- Per-app magic-link login on this app's own origin. -->
                    <a class="signin button" href="/signin">Sign in</a>
                {/if}
            </section>
        </nav>
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
        /* Always collapsed behind the hamburger: hidden by default at every
           width, shown only when the toggle adds `.open`. Rendered as a
           dropdown panel overlaying content (position:absolute) so opening it
           does not reflow the header. */
        display: none;
        position: absolute;
        top: 100%;
        left: 1.5rem;
        z-index: 20;
        flex-direction: column;
        align-items: stretch;
        gap: 0.5rem;
        min-width: 14rem;
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
    .chrome {
        display: flex;
        flex-direction: column;
        align-items: stretch;
        gap: 0.75rem;
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
        padding: 0.375rem 0.5rem;
        font-size: 0.875rem;
        color: var(--mxi-color-fg);
        background: var(--mxi-color-bg, transparent);
        border: 1px solid var(--mxi-color-border);
        border-radius: 0.25rem;
        cursor: pointer;
    }
    main {
        width: 100%;
        padding: 1.5rem;
    }
    .session {
        display: flex;
        flex-direction: column;
        align-items: stretch;
        gap: 0.5rem;
        padding-top: 0.5rem;
        border-top: 1px solid var(--mxi-color-border);
        font-size: 0.85rem;
    }
    .session-title {
        font-weight: 600;
    }
    .session-status {
        margin: 0;
        color: var(--mxi-color-muted, #555);
    }
    .session button {
        padding: 0.3rem 0.5rem;
        border-radius: var(--mxi-radius);
        cursor: pointer;
    }
    .session .signin {
        display: inline-block;
        padding: 0.3rem 0.5rem;
        border-radius: var(--mxi-radius);
        background: var(--mxi-color-primary, #356);
        color: var(--mxi-color-primary-fg, #fff);
        text-decoration: none;
        font-weight: 600;
    }
</style>
