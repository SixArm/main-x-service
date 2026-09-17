<!--
  Root layout — the site shell wrapping every route.

  Renders the persistent top navigation bar (brand, primary nav, the Lily
  PickerBar — theme/locale/text-size/share) and a <main> slot for the
  active page, plus a footer linking back to the monorepo. Pure shell:
  this site has no data fetching, no auth, and no forms — it is a
  read-only public front door onto the main-x-service monorepo, not one
  of the family's operator front-ends.

  Props:
    - children: Snippet — the active route's content, rendered in <main>.
-->
<script lang="ts">
    import { page } from "$app/state";
    import type { Snippet } from "svelte";
    import PickerBar from "@lilydesignsystem/svelte-picker-bar";
    import type { ShareTarget } from "@lilydesignsystem/svelte-share-picker";

    let { children }: { children: Snippet } = $props();

    const navItems = [
        { href: "/", label: "Home" },
        { href: "/architecture/", label: "Architecture" },
        { href: "/subprojects/", label: "Subprojects" },
        { href: "/about/", label: "About" },
    ] as const;

    // This site has no `page.data.title` load-function convention (no
    // per-route load functions at all) — each route's own <svelte:head>
    // sets its title directly. Mirror that same "<label> · Main X Index"
    // shape from navItems for SharePicker, rather than reading the DOM.
    const activeNavLabel = $derived(
        navItems.find((item) => item.href === page.url.pathname)?.label,
    );
    const pageTitle = $derived(
        activeNavLabel ? `${activeNavLabel} · Main X Index` : "Main X Index",
    );

    // This site has no i18n catalogue (all copy is plain English) — same
    // special case as patient-flow-front-end-with-svelte in the family.
    // PickerBar's locale picker still runs fully self-contained: it sets
    // `lang`/`dir` on `<html>` itself (`applyDir` left at its default,
    // `true`), useful to a returning visitor's browser/assistive tech
    // even though the page's own text stays English either way.
    const LOCALES = [
        "en", "en_US", "cy", "es", "fr", "de", "ar", "ru", "hi", "zh", "bn",
        "pt", "id", "ur",
    ];
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

    // Share destinations for the Lily SharePicker — same set as every
    // operator front-end (agents/share/svelte-front-end-stack.md's
    // sibling doc has no equivalent for share targets; this list is
    // repo convention, not a shared constant).
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
</script>

<div class="layout">
    <header class="topbar">
        <a href="/" class="brand">Main X Index</a>
        <nav class="primary-nav">
            <ul>
                {#each navItems as item}
                    <li>
                        <a
                            href={item.href}
                            aria-current={page.url.pathname === item.href ? "page" : null}
                        >
                            {item.label}
                        </a>
                    </li>
                {/each}
            </ul>
        </nav>
        <div class="chrome">
            <PickerBar
                labels={{
                    theme: "Theme",
                    locale: "Language",
                    textSize: "Text size",
                    share: "Share",
                }}
                themesUrl="/assets/themes/"
                themeProps={{ storageKey: "mxi-github-io-theme" }}
                locales={LOCALES}
                localeProps={{ localeLabels: LOCALE_LABELS }}
                textSizeProps={{ storageKey: "mxi-github-io-text-size" }}
                shareTargets={SHARE_TARGETS}
                shareProps={{
                    title: pageTitle,
                    copyLabel: "Copy link",
                    copiedLabel: "Copied",
                    copyFailedLabel: "Could not copy — copy it from the address bar",
                }}
            />
        </div>
    </header>
    <main>
        {@render children()}
    </main>
    <footer>
        <p>
            <a href="https://github.com/SixArm/main-x-service">GitHub</a> ·
            <a href="https://codeberg.org/SixArm/main-x-service">Codeberg</a> ·
            source published from the
            <code>main-x-service.github.io/</code> subproject via
            <code>git subtree</code>.
        </p>
    </footer>
</div>

<style>
    .layout {
        display: flex;
        flex-direction: column;
        min-height: 100vh;
    }
    .topbar {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 1.5rem;
        padding: 0.75rem 1.5rem;
        background: var(--mxi-color-surface);
        border-bottom: 1px solid var(--mxi-color-border);
    }
    .brand {
        font-size: 1.125rem;
        font-weight: 700;
        color: var(--mxi-color-fg);
        white-space: nowrap;
        text-decoration: none;
    }
    .primary-nav {
        flex: 1;
    }
    .primary-nav ul {
        list-style: none;
        display: flex;
        gap: 1.25rem;
        margin: 0;
        padding: 0;
    }
    .primary-nav a {
        color: var(--mxi-color-fg);
        text-decoration: none;
    }
    .primary-nav a:hover {
        color: var(--mxi-color-primary);
    }
    .primary-nav a[aria-current="page"] {
        color: var(--mxi-color-primary);
        font-weight: 600;
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
        background: transparent;
        border: 1px solid var(--mxi-color-border);
        border-radius: 0.25rem;
        cursor: pointer;
    }
    main {
        flex: 1;
        width: 100%;
        max-width: 60rem;
        margin: 0 auto;
        padding: 2rem 1.5rem;
    }
    footer {
        padding: 1.5rem;
        text-align: center;
        font-size: 0.85rem;
        color: var(--mxi-color-muted);
        border-top: 1px solid var(--mxi-color-border);
    }
</style>
