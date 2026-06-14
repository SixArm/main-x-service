<script lang="ts">
    // Root layout — the chrome wrapped around every route.
    //
    // Renders the NHS-themed utility row (locale + theme pickers, the
    // signed-in identity / sign-out control), the branded header with
    // primary navigation, the main content slot, and the footer. The
    // header nav and the auth-status block are hidden while signed out
    // (`user` is null), so the login / callback pages render bare.
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

    import SkipLink from '$lib/components/SkipLink/SkipLink.svelte';
    import Header from '$lib/components/Header/Header.svelte';
    import Footer from '$lib/components/Footer/Footer.svelte';
    import NavigationMenu from '$lib/components/NavigationMenu/NavigationMenu.svelte';

    import LocalePicker from '@lily/locale-picker';
    import ThemePicker from '@lily/theme-picker';

    import { cache } from '$lib/store/cache.svelte';
    import { api } from '$lib/api/client';

    let { children } = $props();

    const user = $derived(cache.user);

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

    const links = [
        { href: '/', label: 'Dashboard' },
        { href: '/patients', label: 'Patients' },
        { href: '/folders', label: 'Folders' },
        { href: '/volumes', label: 'Volumes' },
        { href: '/workers', label: 'Workers' },
        { href: '/buildings', label: 'Buildings' },
        { href: '/cabinets', label: 'Cabinets' },
        { href: '/move', label: 'Move folder' },
        { href: '/scan', label: 'Scan' },
        { href: '/history', label: 'Move history' },
        { href: '/alerts', label: 'Alerts' },
        { href: '/reports', label: 'Reports' }
    ];

    const locales = ['en', 'cy', 'gd'];
    const localeLabels: Record<string, string> = {
        en: 'English',
        cy: 'Cymraeg',
        gd: 'Gàidhlig'
    };

    const themes = ['nhs', 'nhs-high-contrast'];
    const themeLabels: Record<string, string> = {
        nhs: 'Default',
        'nhs-high-contrast': 'High contrast'
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

<SkipLink href="#content" label="Skip to main content" />

<div class="utility-row">
    <div class="page-wrapper">
        <LocalePicker
            label="Language"
            {locales}
            {localeLabels}
            defaultValue="en"
            storageKey="case-folder:locale"
            class="utility-row-picker"
        />
        <ThemePicker
            label="Theme"
            themesUrl="/themes/"
            {themes}
            {themeLabels}
            defaultValue="nhs"
            storageKey="case-folder:theme"
            class="utility-row-picker"
        />
        {#if user}
            <span class="auth-status">
                Signed in as <strong>{user.name}</strong>{#if user.role} ({user.role}){/if}
                <button type="button" class="signout" onclick={signOut}>Sign out</button>
            </span>
        {/if}
    </div>
</div>

<Header label="Site header" class="app-header">
    <div class="page-wrapper">
        <div class="brand">
            <h1>Case Tracking</h1>
            <span class="brand-tag">NHS paper records</span>
        </div>
        {#if user}
            <NavigationMenu label="Primary navigation">
                {#each links as link (link.href)}
                    <a href={link.href} aria-current={isCurrent(link.href) ? 'page' : undefined}>
                        {link.label}
                    </a>
                {/each}
            </NavigationMenu>
        {/if}
    </div>
</Header>

<main id="content" class="page-wrapper">
    {@render children()}
</main>

<Footer label="Site footer" class="app-footer">
    <div class="page-wrapper">
        <p>
            Case Tracking — built with the Lily Design System (NHS theme) and
            SVAR Svelte. Demo data only; not a regulated medical record.
        </p>
    </div>
</Footer>
