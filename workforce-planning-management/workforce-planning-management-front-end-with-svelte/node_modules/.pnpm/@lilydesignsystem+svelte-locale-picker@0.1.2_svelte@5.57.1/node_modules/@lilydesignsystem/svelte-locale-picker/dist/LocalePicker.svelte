<script lang="ts" module>
    import type { Snippet } from "svelte";
    import { IconButton, Listbox } from "@lilydesignsystem/svelte-headless";
    import {
        defaultLocaleLabels,
        RTL_LANGUAGE_TAGS,
        RTL_SCRIPT_SUBTAGS,
    } from "./locales.js";

    /**
     * Default button icon: a bundled SVG (globe outline), not a Unicode
     * character. Reversed 2026-09-16 from the font-dependent-glyph
     * convention (was U+1F310 GLOBE WITH MERIDIANS + U+FE0E, exported as
     * `GLOBE_WITH_MERIDIANS` — removed, not renamed). The old glyph needed
     * VS15 to force text presentation and still risked the colour-emoji
     * font on stacks that ignore the selector; a bundled outline SVG has
     * no such risk and renders identically everywhere, matching the other
     * four picker icons as one monochrome family. Override via `children`,
     * same as before.
     */

    /** Arguments passed to a custom `children` snippet (the button glyph). */
    export type ChildArgs = {
        /** Currently selected locale code (consumer form, not BCP 47-normalised). */
        value: string;
        /** Is the listbox open? */
        open: boolean;
        /** Resolve a locale code to its display label. */
        labelFor: (locale: string) => string;
    };

    /** Public props for LocalePicker. See `spec/index.md` §4 for the contract. */
    export type Props = {
        /** Accessible name for the button and the listbox. */
        label: string;
        /** Available locale codes. */
        locales: string[];
        /** Currently selected locale code. Two-way bindable. */
        value?: string;
        /** Initial locale when nothing else is supplied. */
        defaultValue?: string;
        /** If set, persist the selection to localStorage under this key. */
        storageKey?: string;
        /** Resolve `navigator.languages` to a supported locale on first visit. */
        detectFromNavigator?: boolean;
        /** `name` of the hidden input that carries the value in a form. */
        name?: string;
        /** Element that receives `lang` and `dir`. Defaults to document.documentElement. */
        target?: HTMLElement | null;
        /** If false, the select only writes `lang` and never touches `dir`. */
        applyDir?: boolean;
        /** Optional pretty labels per locale code. */
        localeLabels?: Record<string, string>;
        /** Replaces the default globe icon inside the button. */
        children?: Snippet<[ChildArgs]>;
        /** Called after the control applies a new locale. */
        onChange?: (locale: string) => void;
        /** Extra CSS class on the root. */
        class?: string;
        /** Spread props onto the root element. */
        [key: string]: unknown;
    };

    // ---------------------------------------------------------------
    // Pure helpers (exported so consumers can reuse them)
    // ---------------------------------------------------------------

    /** Convert a locale code to its BCP 47 hyphen form. */
    export function bcp47LocaleTag(locale: string): string {
        return locale.replace(/_/g, "-");
    }

    /** Detect whether a locale is right-to-left. See spec/index.md §5.6. */
    export function isRtlLocale(locale: string): boolean {
        if (!locale) return false;
        const parts = locale.split(/[-_]/);
        for (const part of parts) {
            if (RTL_SCRIPT_SUBTAGS.has(part.toLowerCase())) return true;
        }
        const base = parts[0]?.toLowerCase() ?? "";
        return RTL_LANGUAGE_TAGS.has(base);
    }

    /** Resolve a locale code to its English name via the built-in table. */
    export function localeName(locale: string): string {
        return defaultLocaleLabels[locale] ?? locale;
    }

    /**
     * The language's own name for itself — "de" → "Deutsch", "cy" →
     * "Cymraeg" — from `Intl.DisplayNames` asked *in that language*.
     *
     * Endonyms are the right default for a language menu: the user who
     * needs it most is the one lost in a UI that is not in their
     * language, and they recognise "Cymraeg" where "Welsh" means
     * nothing to them. Deterministic (no `navigator` dependency), so
     * the server and the client render the same label. Returns "" when
     * the runtime has no data — some runtimes echo the tag back instead
     * of failing, and an echo is not a name.
     */
    export function localeEndonym(locale: string): string {
        try {
            const tag = bcp47LocaleTag(locale);
            const dn = new Intl.DisplayNames([tag], { type: "language" });
            const found = dn.of(tag) ?? "";
            return found && found.toLowerCase() !== tag.toLowerCase()
                ? found
                : "";
        } catch {
            return "";
        }
    }

    /** Re-export the built-in label table and RTL sets for convenience. */
    export { defaultLocaleLabels, RTL_LANGUAGE_TAGS, RTL_SCRIPT_SUBTAGS };

    /** Opportunistic Intl.DisplayNames lookup; never throws. */
    function intlDisplayName(locale: string): string {
        try {
            const env =
                typeof navigator !== "undefined" && navigator.language
                    ? navigator.language
                    : "en";
            const dn = new Intl.DisplayNames([env], { type: "language" });
            return dn.of(bcp47LocaleTag(locale)) ?? "";
        } catch {
            return "";
        }
    }

    /** Match a navigator preference against a supported-locales list. */
    export function matchNavigatorLanguage(
        navLangs: readonly string[],
        locales: readonly string[],
    ): string | "" {
        const lc = (s: string) => s.toLowerCase().replace(/_/g, "-");
        const localesLc = locales.map(lc);
        for (const raw of navLangs) {
            const nav = lc(raw);

            // 1. Exact match (treating - and _ as equivalent).
            const exactIndex = localesLc.indexOf(nav);
            if (exactIndex !== -1) return locales[exactIndex];

            // 2. Language-only match: pick the first locale whose
            //    base language matches the navigator's base language.
            const navBase = nav.split("-")[0];
            for (let i = 0; i < locales.length; i++) {
                const base = localesLc[i].split("-")[0];
                if (base === navBase) return locales[i];
            }
        }
        return "";
    }

    let uid = 0;
    /** Stable per-instance id prefix; SSR-safe (no Math.random / Date.now). */
    export function nextLocalePickerId(): string {
        uid += 1;
        return `locale-picker-${uid}`;
    }
</script>

<script lang="ts">
    let {
        class: className = "",
        label,
        locales,
        value = $bindable(""),
        defaultValue,
        storageKey,
        detectFromNavigator = false,
        name = "locale",
        target,
        applyDir = true,
        localeLabels = {},
        children,
        onChange,
        ...restProps
    }: Props = $props();

    const baseId = nextLocalePickerId();
    const listId = `${baseId}-list`;
    const optionId = (i: number) => `${baseId}-option-${i}`;

    let open = $state(false);
    let activeIndex = $state(-1);
    let buttonEl: HTMLButtonElement | undefined = $state();
    let listEl: HTMLElement | undefined = $state();
    let rootEl: HTMLDivElement | undefined = $state();

    function labelFor(locale: string): string {
        if (locale in localeLabels) return localeLabels[locale];
        // Endonym first: a language menu names each language in itself,
        // because the user who needs the menu is the one who cannot read
        // the page's language. The English table and the environment
        // lookup are fallbacks for runtimes without DisplayNames data.
        const endonym = localeEndonym(locale);
        if (endonym) return endonym;
        if (locale in defaultLocaleLabels) return defaultLocaleLabels[locale];
        const intl = intlDisplayName(locale);
        if (intl) return intl;
        return locale;
    }

    /**
     * The `lang` attribute for one option — a claim about the language
     * of the option's TEXT, made only when the text is the endonym we
     * derived ourselves. A consumer label or the English fallback is in
     * whatever language the consumer's UI speaks, and claiming otherwise
     * sends a screen reader's speech engine to the wrong voice: the
     * English word "Arabic" read out by an Arabic synthesizer.
     */
    function optionLang(locale: string): string | undefined {
        if (locale in localeLabels) return undefined;
        return localeEndonym(locale) ? bcp47LocaleTag(locale) : undefined;
    }

    // The code the DOM currently carries. Applying is idempotent: the
    // effect below can run for reasons other than a locale change, and
    // re-applying would re-fire `onChange`. A consumer whose onChange
    // writes reactive state then re-enters this effect, and Svelte stops
    // updating the component altogether (effect_update_depth_exceeded) —
    // the listbox freezes mid-open with a stale aria-expanded. Guarding
    // here also matches the spec: other prop changes are not retroactive.
    let appliedValue = "";

    function applyLocale(code: string): void {
        if (typeof document === "undefined" || !code) return;
        if (code === appliedValue) return;
        appliedValue = code;
        const root = target ?? document.documentElement;
        root.setAttribute("lang", bcp47LocaleTag(code));
        if (applyDir) {
            root.setAttribute("dir", isRtlLocale(code) ? "rtl" : "ltr");
        }
        if (storageKey) {
            try {
                localStorage.setItem(storageKey, code);
            } catch {
                // ignore quota / privacy errors
            }
        }
        onChange?.(code);
    }

    function setLocale(code: string): void {
        value = code;
    }

    // ---------------------------------------------------------------
    // Open / close
    // ---------------------------------------------------------------

    function openList(startIndex?: number): void {
        const selected = locales.indexOf(value);
        // An empty list has no option to activate; -1 keeps
        // aria-activedescendant off rather than pointing at an id that
        // does not exist.
        activeIndex =
            locales.length === 0
                ? -1
                : (startIndex ?? (selected >= 0 ? selected : 0));
        open = true;
        // Focus moves to the listbox; the active option is conveyed via
        // aria-activedescendant, per the APG listbox pattern. preventScroll
        // stops the browser's default scroll-into-view: the listbox is
        // positioned by CSS (see AGENTS/theme.md), and without a consumer
        // override for a right-edge header the box can render partly
        // off-screen, and focusing it then auto-scrolled the whole page --
        // which reads as the page jumping sideways the instant the picker
        // opens.
        queueMicrotask(() => {
            listEl?.focus({ preventScroll: true });
            scrollActiveIntoView();
        });
    }

    function closeList(refocus = true): void {
        if (!open) return;
        open = false;
        activeIndex = -1;
        if (refocus) queueMicrotask(() => buttonEl?.focus({ preventScroll: true }));
    }

    function choose(index: number): void {
        const code = locales[index];
        if (code) setLocale(code);
        closeList();
    }

    function scrollActiveIntoView(): void {
        if (activeIndex < 0 || !listEl) return;
        // getElementById, not a `#id` selector: ids here are generated and
        // contain nothing needing escaping, and `CSS` is absent entirely in
        // jsdom — `CSS.escape` there throws inside the keydown handler,
        // after activeIndex is already assigned, so the suite stays green
        // while this path never actually runs.
        const el = document.getElementById(optionId(activeIndex));
        // Guard the METHOD, not just the element: jsdom implements no
        // scrollIntoView, so `el?.scrollIntoView(...)` throws once `el`
        // exists — and it throws after activeIndex is already assigned,
        // which is why the suite stayed green while this path never ran.
        el?.scrollIntoView?.({ block: "nearest" });
    }

    // Arrow/Home/End/PageUp/PageDown/typeahead/Escape/Tab keyboard handling
    // inside the open list is owned by Listbox's "active-descendant" mode
    // (see @lilydesignsystem/svelte-headless); this component only decides
    // what open/close/choose/scroll mean. Keep the highlighted option in
    // view for every reason activeIndex can change.
    $effect(() => {
        activeIndex;
        scrollActiveIntoView();
    });

    function handleTabOut(): void {
        // Tab moves on — but focus goes to the button FIRST, without
        // cancelling the key (Listbox's onTabOut never preventDefaults
        // Tab). Hiding the focused list drops focus to <body>, and the
        // browser then computes the default Tab move from the top of the
        // document, so tabbing out of an open picker teleported the user to
        // the page's first tab stop. From the button, the default Tab
        // lands exactly where leaving the picker should.
        buttonEl?.focus?.({ preventScroll: true });
        closeList(false);
    }

    function onButtonKeydown(event: KeyboardEvent): void {
        switch (event.key) {
            case "ArrowDown":
            case "Enter":
            case " ":
                event.preventDefault();
                openList();
                break;
            case "ArrowUp":
                event.preventDefault();
                openList(locales.length - 1);
                break;
        }
    }

    function onRootFocusOut(event: FocusEvent): void {
        const next = event.relatedTarget as Node | null;
        if (next && rootEl?.contains(next)) return;
        closeList(false);
    }

    // ---------------------------------------------------------------
    // Initial value resolution + apply (unchanged from the select era)
    // ---------------------------------------------------------------

    let initialised = false;

    $effect(() => {
        const current = value;

        if (!initialised) {
            initialised = true;
            let initial = current;

            if (!initial && storageKey) {
                try {
                    initial = localStorage.getItem(storageKey) ?? "";
                } catch {
                    // ignore privacy errors
                }
            }

            if (!initial && detectFromNavigator && typeof navigator !== "undefined") {
                const navLangs =
                    navigator.languages && navigator.languages.length > 0
                        ? Array.from(navigator.languages)
                        : navigator.language
                          ? [navigator.language]
                          : [];
                initial = matchNavigatorLanguage(navLangs, locales);
            }

            if (!initial) {
                initial =
                    defaultValue ??
                    (locales.includes("en") ? "en" : locales[0]) ??
                    "";
            }

            if (initial && initial !== current) {
                value = initial;
                return;
            }
        }

        if (current) applyLocale(current);
    });
</script>

<svelte:document
    onclick={(event) => {
        if (!open) return;
        const t = event.target as Node | null;
        if (t && rootEl && !rootEl.contains(t)) closeList(false);
    }}
/>

<div
    bind:this={rootEl}
    class={`locale-picker ${className}`.trim()}
    onfocusout={onRootFocusOut}
    {...restProps}
>
    <input type="hidden" {name} {value} />

    <IconButton
        bind:ref={buttonEl}
        baseClass="locale-picker-button"
        label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        onclick={() => (open ? closeList() : openList())}
        onkeydown={onButtonKeydown}
    >
        {#if children}
            {@render children({ value: value ?? "", open, labelFor })}
        {:else}
            <svg
                class="locale-picker-icon"
                viewBox="0 0 16 16"
                width="1.05rem"
                height="1.05rem"
                aria-hidden="true"
                fill="none"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <circle cx="8" cy="8" r="6" />
                <path d="M2 8h12" />
                <path d="M8 2c2.2 0 4 2.7 4 6s-1.8 6-4 6-4-2.7-4-6 1.8-6 4-6z" />
            </svg>
        {/if}
    </IconButton>

    <Listbox
        bind:ref={listEl}
        as="ul"
        baseClass="locale-picker-list"
        id={listId}
        label={label}
        navigation="active-descendant"
        clamp
        typeahead
        pageSize={10}
        bind:activeIndex
        hidden={!open}
        onActivate={choose}
        onEscape={() => closeList()}
        onTabOut={handleTabOut}
    >
        {#each locales as locale, i (locale)}
            <!-- The option's keyboard interaction lives on the listbox
                 (aria-activedescendant pattern): the list is the focused
                 element and its keydown handler operates the options, so a
                 per-option key handler would be wrong, not missing. -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <li
                class="locale-picker-option"
                id={optionId(i)}
                role="option"
                aria-selected={locale === value}
                data-active={i === activeIndex ? "" : undefined}
                lang={optionLang(locale)}
                onclick={() => choose(i)}
            >
                {labelFor(locale)}
            </li>
        {/each}
    </Listbox>
</div>
