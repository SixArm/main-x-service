<script lang="ts">
    // Root layout — the chrome wrapped around every route.
    //
    // Renders the NHS-themed utility row (theme/text-size/share pickers,
    // the signed-in identity / sign-out control), the branded top header
    // with primary navigation (hamburger-collapsible on narrow viewports),
    // the full-width main content slot, and the footer. The primary
    // navigation shows on every real route (including the dashboard at
    // `/`, even before the auth probe resolves or when the API is
    // unreachable); only the `/login` and `/auth/callback` routes render
    // bare (`isBareRoute`). The signed-in identity / sign-out block stays
    // gated on `user`.
    //
    // State:
    //   user — derived from the cache; set by `+layout.ts` after the
    //          `/api/auth/me` probe. Reactive, so signing in/out updates
    //          the chrome without a reload.
    //
    // Side effects:
    //   signOut() — best-effort logout, then clears the cache and routes
    //               to /login regardless of the API outcome.

    import '$lib/css/nhs.css';
    import '$lib/css/app.css';
    import { page } from '$app/state';
    import { goto } from '$app/navigation';
    import { browser } from '$app/environment';

    import SkipLink from '$lib/components/SkipLink/SkipLink.svelte';
    import Header from '$lib/components/Header/Header.svelte';
    import Footer from '$lib/components/Footer/Footer.svelte';
    import NavigationMenu from '$lib/components/NavigationMenu/NavigationMenu.svelte';

    import ThemePicker from 'lily-design-system-svelte-theme-picker';
    import {
        SharePicker,
        type ShareTarget,
    } from 'lily-design-system-svelte-share-picker';
    import { TextSizePicker } from 'lily-design-system-svelte-text-size-picker';

    import { cache } from '$lib/store/cache.svelte';
    import { api } from '$lib/api/client';
    import { i18n, t, isRtl, type StringKey } from '$lib/i18n.svelte';

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
    // ar/ur, ltr otherwise). SSR-guarded so a load-time render never touches
    // the DOM.
    $effect(() => {
        if (!browser) return;
        const locale = i18n.locale;
        document.documentElement.setAttribute('lang', locale);
        document.documentElement.setAttribute(
            'dir',
            isRtl(locale) ? 'rtl' : 'ltr',
        );
    });

    // Text sizes offered by the Lily TextSizePicker. Applied as
    // `data-text-size` on <html> (attribute-based, mirroring ThemePicker's
    // `data-theme`); see app.css for the corresponding font-size scale.
    const SIZES = ['small', 'medium', 'large', 'x-large'];
    const SIZE_LABELS: Record<string, string> = {
        small: 'Small',
        medium: 'Medium',
        large: 'Large',
        'x-large': 'Extra large',
    };

    // Share destinations for the Lily SharePicker. Lily ships no
    // third-party URLs — each `href` builder is ours. `url`/`title` are
    // supplied by SharePicker at share time (current page URL; the leaf
    // page's title, sourced from `page.data.title` below — the
    // `page.data.title` convention, set per-route by each route's load
    // function so it stays in sync with that page's own content without
    // SharePicker having to read the DOM).
    const SHARE_TARGETS: ShareTarget[] = [
        {
            id: 'linkedin',
            label: 'LinkedIn',
            href: (url) =>
                `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(url)}`,
        },
        {
            id: 'mastodon',
            label: 'Mastodon',
            href: (url, title) =>
                `https://mastodon.social/share?text=${encodeURIComponent(`${title} ${url}`)}`,
        },
        {
            id: 'bluesky',
            label: 'Bluesky',
            href: (url, title) =>
                `https://bsky.app/intent/compose?text=${encodeURIComponent(`${title} ${url}`)}`,
        },
        {
            id: 'reddit',
            label: 'Reddit',
            href: (url, title) =>
                `https://www.reddit.com/submit?url=${encodeURIComponent(url)}&title=${encodeURIComponent(title)}`,
        },
    ];

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
    ];

    // Full Lily/DaisyUI theme catalogue (copied verbatim from the sibling
    // course-front-end-with-svelte for family parity). Each slug has a Lily
    // stylesheet at `static/assets/themes/<slug>.css` (a symlink to the shared
    // design-system themes) that ThemePicker swaps in. Includes the NHS
    // England/Scotland/Wales patient & practitioner themes; the
    // practitioner-facing English theme is the sensible NHS default.
    const themes = [
        'abyss',
        'acid',
        'aqua',
        'autumn',
        'black',
        'bumblebee',
        'business',
        'caramellatte',
        'cmyk',
        'coffee',
        'corporate',
        'cupcake',
        'cyberpunk',
        'dark',
        'dim',
        'dracula',
        'emerald',
        'fantasy',
        'forest',
        'garden',
        'halloween',
        'lemonade',
        'light',
        'lofi',
        'luxury',
        'night',
        'nord',
        'pastel',
        'retro',
        'silk',
        'sunset',
        'synthwave',
        'united-kingdom-national-health-service-england-for-patients',
        'united-kingdom-national-health-service-england-for-practitioners',
        'united-kingdom-national-health-service-scotland-for-patients',
        'united-kingdom-national-health-service-scotland-for-practitioners',
        'united-kingdom-national-health-service-wales-for-patients',
        'united-kingdom-national-health-service-wales-for-practitioners',
        'valentine',
        'winter',
        'wireframe',
    ];

    // Human-readable labels for the theme select — the FULL theme name for
    // each slug (DaisyUI names title-cased; the NHS slugs spelled out in full).
    const THEME_LABELS: Record<string, string> = {
        abyss: 'Abyss',
        acid: 'Acid',
        aqua: 'Aqua',
        autumn: 'Autumn',
        black: 'Black',
        bumblebee: 'Bumblebee',
        business: 'Business',
        caramellatte: 'Caramellatte',
        cmyk: 'Cmyk',
        coffee: 'Coffee',
        corporate: 'Corporate',
        cupcake: 'Cupcake',
        cyberpunk: 'Cyberpunk',
        dark: 'Dark',
        dim: 'Dim',
        dracula: 'Dracula',
        emerald: 'Emerald',
        fantasy: 'Fantasy',
        forest: 'Forest',
        garden: 'Garden',
        halloween: 'Halloween',
        lemonade: 'Lemonade',
        light: 'Light',
        lofi: 'Lofi',
        luxury: 'Luxury',
        night: 'Night',
        nord: 'Nord',
        pastel: 'Pastel',
        retro: 'Retro',
        silk: 'Silk',
        sunset: 'Sunset',
        synthwave: 'Synthwave',
        valentine: 'Valentine',
        winter: 'Winter',
        wireframe: 'Wireframe',
        'united-kingdom-national-health-service-england-for-patients':
            'United Kingdom National Health Service England for Patients',
        'united-kingdom-national-health-service-england-for-practitioners':
            'United Kingdom National Health Service England for Practitioners',
        'united-kingdom-national-health-service-scotland-for-patients':
            'United Kingdom National Health Service Scotland for Patients',
        'united-kingdom-national-health-service-scotland-for-practitioners':
            'United Kingdom National Health Service Scotland for Practitioners',
        'united-kingdom-national-health-service-wales-for-patients':
            'United Kingdom National Health Service Wales for Patients',
        'united-kingdom-national-health-service-wales-for-practitioners':
            'United Kingdom National Health Service Wales for Practitioners',
    };

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

<div class="utility-row">
    <div class="page-wrapper">
        <ThemePicker
            label={t('chrome.theme')}
            themesUrl="/assets/themes/"
            {themes}
            themeLabels={THEME_LABELS}
            defaultValue="united-kingdom-national-health-service-england-for-practitioners"
            storageKey="case-folder:theme"
            class="utility-row-picker"
        />
        <TextSizePicker
            label={t('nav.text_size')}
            sizes={SIZES}
            sizeLabels={SIZE_LABELS}
            defaultValue="medium"
            storageKey="case-folder:text-size"
            class="utility-row-picker"
        />
        <SharePicker
            label={t('nav.share')}
            title={pageTitle}
            targets={SHARE_TARGETS}
            copyLabel={t('share.copy_link')}
            copiedLabel={t('share.copied')}
            copyFailedLabel={t('share.copy_failed')}
            class="utility-row-picker"
        />
        {#if user}
            <span class="auth-status">
                {t('auth.signedInAs')}
                <strong>{user.name}</strong>{#if user.role}
                    ({user.role}){/if}
                <button type="button" class="signout" onclick={signOut}
                    >{t('auth.signOut')}</button
                >
            </span>
        {/if}
    </div>
</div>

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
            >
                <span class="nav-toggle-box" aria-hidden="true"></span>
            </button>
        {/if}
        <div class="brand">
            <h1>{t('brand.name')}</h1>
            <span class="brand-tag">{t('brand.tagline')}</span>
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
                        onclick={() => (menuOpen = false)}
                    >
                        {t(link.key)}
                    </a>
                {/each}
            </NavigationMenu>
        {/if}
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
