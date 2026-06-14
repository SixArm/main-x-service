// Lightweight, dependency-free i18n for the auth SPA. A per-locale
// strings map plus a reactive `$state` current-locale (Svelte 5 runes),
// exposed via a `t(key)` accessor. Deliberately no i18n library: the
// surface is tiny and we keep the front-end dependency-light.
//
// Supported locales mirror the service catalog (authentication-service
// `src/i18n.rs`): English (`en`) + Welsh (`cy`), Welsh chosen for the
// public-sector Welsh-language duty. Unknown key/locale falls back to
// `en`. The chosen locale persists to localStorage and is sent as the
// `locale` field on signup / magic-link requests so the email language
// matches the UI.

import { browser } from "$app/environment";

/**
 * Locales for which the UI is translated. To add one, extend this tuple
 * AND add a matching entry to {@link LOCALE_LABELS} and `STRINGS`.
 */
/// Locales for which the UI is translated. Extend this + `STRINGS`.
export const LOCALES = ["en", "cy"] as const;
/** A supported locale code (one of {@link LOCALES}). */
export type Locale = (typeof LOCALES)[number];

/** Fallback locale for an unknown key, locale, or missing translation. */
/// Fallback locale for an unknown key, locale, or missing translation.
export const DEFAULT_LOCALE: Locale = "en";

/** Human-readable name for the locale switcher, written in that locale. */
/// Human-readable name for the locale switcher (in that locale).
export const LOCALE_LABELS: Record<Locale, string> = {
    en: "English",
    cy: "Cymraeg",
};

// localStorage key under which the chosen UI locale is persisted.
const LOCALE_KEY = "mxi.auth.locale";

// Every translatable UI string, keyed by a stable dotted key. `en` is the
// source of truth; every other locale must cover the same key set so a
// missing translation is a type error (the `StringKey` union below).
/// Every translatable UI string, keyed by a stable dotted key. `en` is
/// the source of truth; other locales must cover the same keys.
const STRINGS = {
    en: {
        "brand": "Main X Auth",
        "nav.home": "Home",
        "nav.signin": "Sign in",
        "nav.signup": "Sign up",
        "nav.locale": "Language",
        "session.signedInAs": "Signed in as",
        // Home / account
        "account.title": "Account",
        "account.loading": "Loading…",
        "account.name": "Name:",
        "account.email": "Email:",
        "account.id": "ID:",
        "account.signout": "Sign out",
        "account.notSignedIn": "You are not signed in.",
        "account.signinPrompt.signin": "Sign in",
        "account.signinPrompt.or": "or",
        "account.signinPrompt.create": "create an account",
        "account.loadFailed": "Failed to load profile",
        // Sign in
        "signin.title": "Sign in",
        "signin.email": "Email",
        "signin.submit": "Email me a magic link",
        "signin.submitting": "Sending…",
        "signin.sent":
            "If that email has an account, a magic link is on its way. In development the link is printed to the auth service console — open it to sign in.",
        "signin.noAccount": "No account yet?",
        "signin.create": "Create one",
        "signin.failed": "Request failed",
        // Sign up
        "signup.title": "Create account",
        "signup.email": "Email",
        "signup.name": "Name",
        "signup.nameOptional": "(optional)",
        "signup.submit": "Send magic link",
        "signup.submitting": "Sending…",
        "signup.sent":
            "If that email is valid, a magic link is on its way. In development the link is printed to the auth service console — open it to finish signing in.",
        "signup.backToSignin": "Back to sign in",
        "signup.haveAccount": "Already have an account?",
        "signup.signin": "Sign in",
        "signup.failed": "Sign up failed",
        // Verify
        "verify.working.title": "Signing you in…",
        "verify.working.body": "Verifying your magic link.",
        "verify.error.title": "Could not sign you in",
        "verify.error.missingToken": "This link is missing its token.",
        "verify.error.invalid": "This link is invalid or expired.",
        "verify.error.requestNew": "Request a new link",
    },
    cy: {
        "brand": "Main X Auth",
        "nav.home": "Hafan",
        "nav.signin": "Mewngofnodi",
        "nav.signup": "Cofrestru",
        "nav.locale": "Iaith",
        "session.signedInAs": "Wedi mewngofnodi fel",
        // Home / account
        "account.title": "Cyfrif",
        "account.loading": "Yn llwytho…",
        "account.name": "Enw:",
        "account.email": "E-bost:",
        "account.id": "ID:",
        "account.signout": "Allgofnodi",
        "account.notSignedIn": "Nid ydych wedi mewngofnodi.",
        "account.signinPrompt.signin": "Mewngofnodi",
        "account.signinPrompt.or": "neu",
        "account.signinPrompt.create": "creu cyfrif",
        "account.loadFailed": "Methwyd â llwytho'r proffil",
        // Sign in
        "signin.title": "Mewngofnodi",
        "signin.email": "E-bost",
        "signin.submit": "E-bostiwch ddolen hud ataf",
        "signin.submitting": "Yn anfon…",
        "signin.sent":
            "Os oes cyfrif gan yr e-bost hwnnw, mae dolen hud ar ei ffordd. Wrth ddatblygu, argreffir y ddolen i gonsol y gwasanaeth dilysu — agorwch hi i fewngofnodi.",
        "signin.noAccount": "Dim cyfrif eto?",
        "signin.create": "Crëwch un",
        "signin.failed": "Methodd y cais",
        // Sign up
        "signup.title": "Creu cyfrif",
        "signup.email": "E-bost",
        "signup.name": "Enw",
        "signup.nameOptional": "(dewisol)",
        "signup.submit": "Anfon dolen hud",
        "signup.submitting": "Yn anfon…",
        "signup.sent":
            "Os yw'r e-bost hwnnw'n ddilys, mae dolen hud ar ei ffordd. Wrth ddatblygu, argreffir y ddolen i gonsol y gwasanaeth dilysu — agorwch hi i orffen mewngofnodi.",
        "signup.backToSignin": "Yn ôl i fewngofnodi",
        "signup.haveAccount": "Mae gennych gyfrif eisoes?",
        "signup.signin": "Mewngofnodi",
        "signup.failed": "Methodd y cofrestru",
        // Verify
        "verify.working.title": "Yn eich mewngofnodi…",
        "verify.working.body": "Yn gwirio eich dolen hud.",
        "verify.error.title": "Methwyd â'ch mewngofnodi",
        "verify.error.missingToken": "Mae'r ddolen hon yn colli ei thocyn.",
        "verify.error.invalid": "Mae'r ddolen hon yn annilys neu wedi dod i ben.",
        "verify.error.requestNew": "Gofyn am ddolen newydd",
    },
} as const;

/** The set of valid translation keys (derived from the English catalog). */
/// The set of valid string keys (derived from the English catalog).
export type StringKey = keyof (typeof STRINGS)["en"];

// Normalise raw input to a supported locale, or null if unsupported.
// Accepts a region subtag (cy-GB → cy) and is case-insensitive.
/// Normalise raw input to a supported locale, or `null` if unsupported.
/// Accepts a region subtag (`cy-GB` → `cy`) and is case-insensitive.
function normaliseLocale(raw: string | null | undefined): Locale | null {
    if (!raw) return null;
    // Take the primary subtag before any `-`/`_`, lowercased.
    const primary = raw.trim().split(/[-_]/)[0]?.toLowerCase() ?? "";
    return (LOCALES as readonly string[]).includes(primary) ? (primary as Locale) : null;
}

// Seed the reactive locale from localStorage (default off the browser).
function readStoredLocale(): Locale {
    if (!browser) return DEFAULT_LOCALE;
    return normaliseLocale(localStorage.getItem(LOCALE_KEY)) ?? DEFAULT_LOCALE;
}

// Reactive current-locale state; mutating it re-renders every `t(...)` call.
let current = $state<Locale>(readStoredLocale());

/**
 * Reactive current-locale store with persistence.
 *
 * Reading `i18n.locale` (or calling {@link t}) inside a component subscribes
 * that component to locale changes, so switching the locale re-renders the UI.
 */
/// Reactive current-locale + persistence. Reading `i18n.locale` in a
/// component subscribes it to locale changes.
export const i18n = {
    /** The currently selected locale. */
    get locale(): Locale {
        return current;
    },
    /**
     * Switch the active locale and persist the choice.
     *
     * @param next - Desired locale (a region subtag like `cy-GB` is
     *   accepted); unsupported input falls back to {@link DEFAULT_LOCALE}.
     */
    /// Switch locale. Unknown input falls back to the default.
    set(next: string): void {
        const locale = normaliseLocale(next) ?? DEFAULT_LOCALE;
        current = locale;
        if (browser) localStorage.setItem(LOCALE_KEY, locale);
    },
    /** The list of supported locales (for rendering the switcher). */
    get locales(): readonly Locale[] {
        return LOCALES;
    },
};

/**
 * Translate `key` in `locale` with graceful fallbacks.
 *
 * Falls back to the English translation when the target locale lacks the
 * key, then to the key string itself if even English lacks it. Pure — safe
 * to unit-test without a Svelte component (does not read reactive state
 * unless `locale` is defaulted to the current locale).
 *
 * @param key - A valid {@link StringKey}.
 * @param locale - Target locale; defaults to the current reactive locale.
 * @returns The translated string (or the key as a last resort).
 */
/// Translate `key` in `locale` (defaults to the current locale), falling
/// back to `en` for a missing translation, then to the key itself for an
/// unknown key. Pure — safe to unit-test without a component.
export function translate(key: StringKey, locale: Locale = current): string {
    // Unknown locale → English table; unknown key → English → the key.
    const table = STRINGS[locale] ?? STRINGS[DEFAULT_LOCALE];
    return table[key] ?? STRINGS[DEFAULT_LOCALE][key] ?? key;
}

/**
 * Reactive translation accessor for components: `t("signin.title")`.
 *
 * Reads the current reactive locale, so a locale switch re-renders every
 * string rendered through it.
 *
 * @param key - A valid {@link StringKey}.
 * @returns The translated string in the current locale.
 */
/// Reactive accessor for use in components: `t("signin.title")`. Reads
/// the current locale, so a locale switch re-renders the string.
export function t(key: StringKey): string {
    return translate(key, current);
}
