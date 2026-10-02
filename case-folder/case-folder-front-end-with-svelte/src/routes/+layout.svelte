<script lang="ts">
    // Root layout — the chrome wrapped around every route.
    //
    // Renders the branded top header: hamburger-collapsible primary
    // navigation, the brand, and (always visible, at the header's end) a
    // Sign in link or Sign out button followed by the Lily PickerBar —
    // theme, locale, text-size, and share pickers as one row. Then the
    // full-width main content slot and the footer. The primary navigation
    // shows on every real route (including the home page at `/`, even
    // before the auth probe resolves or when the API is unreachable); only
    // the `/login` and `/auth/callback` routes render bare (`isBareRoute`).
    // The signed-in identity and Sign out control are gated on `user`.
    //
    // State:
    //   user — derived from the cache; set by `+layout.ts` after the
    //          `/api/auth/me` probe. Reactive, so signing in/out updates
    //          the chrome without a reload.
    //
    // Side effects:
    //   signOut() — best-effort logout, then clears the cache and routes
    //               to /login regardless of the API outcome.

    import '#lib/css/nhs.css';
    import '#lib/css/app.css';
    import { page } from '$app/state';
    import { goto } from '$app/navigation';
    import { browser } from '$app/env';

    import SkipLink from '#lib/components/SkipLink/SkipLink.svelte';
    import Header from '#lib/components/Header/Header.svelte';
    import Footer from '#lib/components/Footer/Footer.svelte';
    import NavigationMenu from '#lib/components/NavigationMenu/NavigationMenu.svelte';

    import PickerBar from '@lilydesignsystem/svelte-picker-bar';
    import type { ShareTarget } from '@lilydesignsystem/svelte-share-picker';

    import { cache } from '#lib/store/cache.svelte.js';
    import { api } from '#lib/api/client.js';
    import {
        i18n,
        t,
        isRtl,
        LOCALES,
        LOCALE_LABELS,
        type StringKey,
    } from '#lib/i18n.svelte.js';

    let { children } = $props();

    const user = $derived(cache.user);

    // The login / auth-callback routes render bare (no primary navigation).
    // Everywhere else — including the dashboard at `/` — shows the top nav,
    // regardless of whether the auth probe has resolved a user yet, so the
    // chrome is present even when the API is unreachable in local dev.
    const isBareRoute = $derived(
        ['/login', '/auth/callback'].some(
            (p) =>
                page.url.pathname === p ||
                page.url.pathname.startsWith(`${p}/`),
        ),
    );

    // The i18n store is the single source of truth for the locale: this
    // effect mirrors the chosen locale onto <html lang>/<html dir> (rtl for
    // ar-001, ltr otherwise). SSR-guarded so a load-time render never touches
    // the DOM.
    $effect(() => {
        if (!browser) return;
        const locale = i18n.locale;
        // `i18n.locale` is already a BCP 47 tag ("en-001", "zh-cn"), which
        // is what PickerBar's LocalePicker writes too, so they agree.
        document.documentElement.setAttribute('lang', locale);
        document.documentElement.setAttribute(
            'dir',
            isRtl(locale) ? 'rtl' : 'ltr',
        );
    });

    // Share destinations for the Lily SharePicker. Lily ships no
    // third-party URLs — each `href` builder is ours. `url`/`title` are
    // supplied by SharePicker at share time (current page URL; the leaf
    // page's title, sourced from `page.data.title` below — the
    // `page.data.title` convention, set per-route by each route's load
    // function so it stays in sync with that page's own content without
    // SharePicker having to read the DOM).
    const SHARE_TARGETS: ShareTarget[] = $derived([
        {
            id: 'email',
            label: t('share.email'),
            href: (url, title) =>
                `mailto:?subject=${encodeURIComponent(title)}&body=${encodeURIComponent(url)}`,
            newTab: false,
        },
        {
            id: 'linkedin',
            label: t('share.linkedin'),
            href: (url) =>
                `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(url)}`,
        },
        {
            id: 'reddit',
            label: t('share.reddit'),
            href: (url, title) =>
                `https://www.reddit.com/submit?url=${encodeURIComponent(url)}&title=${encodeURIComponent(title)}`,
        },
        {
            id: 'bluesky',
            label: t('share.bluesky'),
            href: (url, title) =>
                `https://bsky.app/intent/compose?text=${encodeURIComponent(`${title} ${url}`)}`,
        },
        {
            id: 'mastodon',
            label: t('share.mastodon'),
            href: (url, title) =>
                `https://mastodonshare.com/?text=${encodeURIComponent(title)}&url=${encodeURIComponent(url)}`,
        },
    ]);

    // The `page.data.title` convention: each route's own load function
    // (`+page.ts`/`+page.server.ts`) returns a plain `title` string that
    // mirrors what that route renders as its heading, so the layout —
    // which does not know which leaf page is active — can read it here
    // for SharePicker without scraping `document.title`. Falls back to
    // the brand name for the rare route that sets none.
    const pageTitle = $derived(page.data?.title ?? t('brand.name'));

    // Hamburger toggle state for the header navigation (narrow viewports).
    let menuOpen = $state(false);

    // End the session. Clear local auth state and redirect even if the
    // logout call fails, so a flaky API can never strand a signed-in UI.
    async function signOut() {
        try {
            await api.auth.logout();
        } finally {
            cache.clearUser();
            await goto('/login');
        }
    }

    // Nav links carry a translation key; labels are resolved reactively in
    // the template via t(), so switching locale relabels the whole menu.
    const links: { href: string; key: StringKey }[] = [
        { href: '/', key: 'nav.dashboard' },
        { href: '/patients', key: 'nav.patients' },
        { href: '/folders', key: 'nav.folders' },
        { href: '/volumes', key: 'nav.volumes' },
        { href: '/workers', key: 'nav.workers' },
        { href: '/buildings', key: 'nav.buildings' },
        { href: '/cabinets', key: 'nav.cabinets' },
        { href: '/move', key: 'nav.move' },
        { href: '/scan', key: 'nav.scan' },
        { href: '/history', key: 'nav.history' },
        { href: '/alerts', key: 'nav.alerts' },
        { href: '/reports', key: 'nav.reports' },
        { href: '/tour', key: 'nav.tour' },
    ];

    // Mark a nav link as the current page for `aria-current`. The
    // dashboard link must match exactly (every path starts with '/'),
    // while section links match any of their sub-paths (e.g. /folders/new
    // keeps "Folders" highlighted).
    function isCurrent(href: string): boolean {
        if (href === '/') return page.url.pathname === '/';
        return page.url.pathname.startsWith(href);
    }
</script>

<SkipLink href="#content" label={t('layout.skipToContent')} />

<Header label={t('layout.siteHeader')} class="app-header">
    <div class="page-wrapper">
        {#if !isBareRoute}
            <button
                type="button"
                class="nav-toggle"
                aria-expanded={menuOpen}
                aria-controls="primary-navigation"
                aria-label={t('nav.toggle')}
                onclick={() => (menuOpen = !menuOpen)}
                ><span class="nav-toggle-box" aria-hidden="true"></span></button
            >
        {/if}
        <div class="brand">
            <h1>{t('brand.name')}</h1>
            <span class="brand-tag tagline">{t('brand.tagline')}</span>
        </div>
        {#if !isBareRoute}
            <NavigationMenu
                label={t('layout.primaryNavigation')}
                id="primary-navigation"
                class={menuOpen ? 'open' : ''}
            >
                {#each links as link (link.href)}
                    <a
                        href={link.href}
                        aria-current={isCurrent(link.href) ? 'page' : undefined}
                        onclick={() => (menuOpen = false)}>{t(link.key)}</a
                    >
                {/each}
            </NavigationMenu>
        {/if}
        <div class="header-end">
            {#if user}
                <span class="auth-status">
                    <strong>{user.name}</strong>{#if user.role}
                        ({user.role}){/if}
                </span>
                <button type="button" class="session-button" onclick={signOut}
                    >{t('auth.signout')}</button
                >
            {:else if !isBareRoute}
                <a class="session-button signin" href="/login"
                    >{t('auth.signin')}</a
                >
            {/if}
            <PickerBar
                labels={{
                    theme: t('chrome.theme'),
                    locale: t('chrome.language'),
                    textSize: t('nav.text_size'),
                    share: t('nav.share'),
                }}
                themesUrl="/assets/themes/"
                themeProps={{
                    storageKey: 'lily-theme',
                    // Without a default no theme stylesheet loads until the
                    // user picks one, leaving the pickers (which Lily's
                    // theme CSS styles) unstyled on first visit. Follow the
                    // OS light/dark preference, else fall back to "light".
                    detectFromSystem: true,
                    defaultValue: 'light',
                }}
                locales={[...LOCALES]}
                localeProps={{
                    value: i18n.locale,
                    localeLabels: LOCALE_LABELS,
                    applyDir: false,
                    onChange: (code: string) => i18n.set(code),
                }}
                textSizeProps={{
                    storageKey: 'case-folder:text-size',
                }}
                shareTargets={SHARE_TARGETS}
                shareProps={{
                    title: pageTitle,
                    copyLabel: t('share.copy_link'),
                    copiedLabel: t('share.copied'),
                    copyFailedLabel: t('share.copy_failed'),
                }}
            />
        </div>
    </div>
</Header>

<main id="content" class="page-content">
    {@render children()}
</main>

<Footer label={t('layout.siteFooter')} class="app-footer">
    <div class="page-wrapper">
        <p>{t('footer.text')}</p>
    </div>
</Footer>
