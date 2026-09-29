<script lang="ts" module>
    import type { Snippet } from "svelte";

    /**
     * Default button icon: a bundled SVG (outline right arrow), not a
     * Unicode character. Reversed 2026-09-16 from the font-dependent-glyph
     * convention (was U+27A4 BLACK RIGHTWARDS ARROWHEAD, exported as
     * `BLACK_RIGHTWARDS_ARROWHEAD` — removed, not renamed). Maintainer-
     * directed, following the outline-arrow icon already used at
     * https://testingexamples.github.io/. A bundled SVG renders identically
     * across every font stack; the other four picker icons moved to the
     * same bundled-SVG convention the same day.
     */

    /**
     * One destination in the share list.
     *
     * `href` is a function, not a string, so the consumer owns the whole URL
     * — this package ships no third-party endpoints and takes no view on
     * which networks exist. See `spec/index.md` §3.
     */
    export type ShareTarget = {
        /** Stable identifier, passed back to `onShare`. */
        id: string;
        /** Visible link text. Consumer-supplied, so it localises. */
        label: string;
        /** Build the destination URL from the shared page's metadata. */
        href: (url: string, title: string, text: string) => string;
        /** Overrides the default `target="_blank"` for this destination. */
        newTab?: boolean;
    };

    /** Arguments passed to a custom `children` snippet (the button glyph). */
    export type ChildArgs = {
        /** Is the list open? */
        open: boolean;
        /** The URL that would be shared right now. */
        url: string;
    };

    /** How the button behaves when activated. */
    export type ShareStrategy = "auto" | "native" | "list";

    /** Public props for SharePicker. See `spec/index.md` §4 for the contract. */
    export type Props = {
        /** Accessible name for the button and the list. */
        label: string;
        /** Destinations to offer. Empty is valid if `copyLabel` is set. */
        targets?: ShareTarget[];
        /** URL to share. Defaults to the current page URL, read at share time. */
        url?: string;
        /** Title passed to `href(...)` and to the native share sheet. */
        title?: string;
        /** Longer text passed to `href(...)` and to the native share sheet. */
        text?: string;
        /**
         * Label for the built-in copy-to-clipboard item. The item renders
         * only when this is supplied — there is no default, because a
         * default would be a hardcoded English string.
         */
        copyLabel?: string;
        /** Announced in the status region after a successful copy. */
        copiedLabel?: string;
        /** Announced in the status region when the clipboard write fails. */
        copyFailedLabel?: string;
        /**
         * `"auto"` (default) uses the native share sheet when the browser
         * provides one and falls back to the list; `"native"` always tries
         * the sheet; `"list"` always shows the list.
         */
        strategy?: ShareStrategy;
        /** Replaces the default arrow icon inside the button. */
        children?: Snippet<[ChildArgs]>;
        /** Fires after a destination is chosen, with its `id`. */
        onShare?: (targetId: string, url: string) => void;
        /** Fires after the URL is copied. */
        onCopy?: (url: string) => void;
        /** Fires when the native share sheet is used, instead of the list. */
        onNativeShare?: (url: string) => void;
        /** Extra CSS class on the root. */
        class?: string;
        /** Spread props onto the root element. */
        [key: string]: unknown;
    };

    /** Is a native share sheet available? SSR-safe. */
    export function canShareNatively(): boolean {
        return typeof navigator !== "undefined" && typeof navigator.share === "function";
    }

    /** Is an async clipboard available? SSR-safe. */
    export function canCopy(): boolean {
        return (
            typeof navigator !== "undefined" &&
            typeof navigator.clipboard?.writeText === "function"
        );
    }

    let uid = 0;
    /** Stable per-instance id prefix; SSR-safe (no Math.random / Date.now). */
    export function nextSharePickerId(): string {
        uid += 1;
        return `share-picker-${uid}`;
    }
</script>

<script lang="ts">
    let {
        class: className = "",
        label,
        targets = [],
        url,
        title = "",
        text = "",
        copyLabel,
        copiedLabel,
        copyFailedLabel,
        strategy = "auto",
        children,
        onShare,
        onCopy,
        onNativeShare,
        ...restProps
    }: Props = $props();

    const baseId = nextSharePickerId();
    const listId = `${baseId}-list`;

    let open = $state(false);
    let status = $state("");
    let buttonEl: HTMLButtonElement | undefined = $state();
    let listEl: HTMLUListElement | undefined = $state();
    let rootEl: HTMLDivElement | undefined = $state();

    /**
     * The URL to share. Resolved lazily so the default works without the
     * consumer threading `location.href` through, and so SSR never touches
     * `location`.
     */
    function currentUrl(): string {
        if (url) return url;
        return typeof location !== "undefined" ? location.href : "";
    }

    /** Every focusable item in the list, in DOM order. */
    function items(): HTMLElement[] {
        if (!listEl) return [];
        return Array.from(
            listEl.querySelectorAll<HTMLElement>(".share-picker-target, .share-picker-copy"),
        );
    }

    function openList(focusLast = false): void {
        open = true;
        status = "";
        // preventScroll stops the browser's default scroll-into-view: the
        // list is positioned by CSS (see AGENTS/theme.md), and without a
        // consumer override for a right-edge header it can render partly
        // off-screen, and focusing an item then auto-scrolled the whole
        // page -- which reads as the page jumping sideways the instant the
        // picker opens.
        queueMicrotask(() => {
            const all = items();
            (focusLast ? all[all.length - 1] : all[0])?.focus({ preventScroll: true });
        });
    }

    function closeList(refocus = true): void {
        if (!open) return;
        open = false;
        if (refocus) queueMicrotask(() => buttonEl?.focus({ preventScroll: true }));
    }

    async function shareNatively(): Promise<boolean> {
        if (!canShareNatively()) return false;
        const shareUrl = currentUrl();
        try {
            await navigator.share({ url: shareUrl, title, text });
            onNativeShare?.(shareUrl);
            return true;
        } catch {
            // A rejected promise here is almost always the user dismissing
            // the sheet, which is not an error and must not fall through to
            // the list — that would reopen UI they just dismissed.
            return true;
        }
    }

    async function onButtonClick(): Promise<void> {
        if (open) {
            closeList();
            return;
        }
        if (strategy === "native" || (strategy === "auto" && canShareNatively())) {
            if (await shareNatively()) return;
        }
        openList();
    }

    function onButtonKeydown(event: KeyboardEvent): void {
        // Enter and Space are the button's own activation keys and already
        // produce a click; only the arrows need handling here.
        if (event.key === "ArrowDown") {
            event.preventDefault();
            if (!open) openList();
            else items()[0]?.focus({ preventScroll: true });
        } else if (event.key === "ArrowUp") {
            event.preventDefault();
            if (!open) openList(true);
            else items()[items().length - 1]?.focus({ preventScroll: true });
        }
    }

    function moveFocus(delta: number): void {
        const all = items();
        if (all.length === 0) return;
        const i = all.indexOf(document.activeElement as HTMLElement);
        const next = Math.min(Math.max((i < 0 ? 0 : i) + delta, 0), all.length - 1);
        all[next]?.focus({ preventScroll: true });
    }

    function onListKeydown(event: KeyboardEvent): void {
        switch (event.key) {
            case "ArrowDown":
                event.preventDefault();
                moveFocus(1);
                break;
            case "ArrowUp":
                event.preventDefault();
                moveFocus(-1);
                break;
            case "Home":
                event.preventDefault();
                items()[0]?.focus({ preventScroll: true });
                break;
            case "End": {
                event.preventDefault();
                const all = items();
                all[all.length - 1]?.focus({ preventScroll: true });
                break;
            }
            case "Escape":
                event.preventDefault();
                closeList();
                break;
            case "Tab":
                // Tab leaves the control — but focus goes to the button
                // FIRST, without cancelling the key. Hiding the list while
                // one of its items has focus drops focus to <body>, and
                // the browser then computes the default Tab move from the
                // top of the document, so tabbing out of the open list
                // teleported the user to the page's first tab stop. From
                // the button, the default Tab lands exactly where leaving
                // the picker should.
                buttonEl?.focus?.({ preventScroll: true });
                closeList(false);
                break;
        }
    }

    function onRootFocusOut(event: FocusEvent): void {
        const next = event.relatedTarget as Node | null;
        if (next && rootEl?.contains(next)) return;
        closeList(false);
    }

    function chooseTarget(target: ShareTarget): void {
        onShare?.(target.id, currentUrl());
        closeList();
    }

    async function copyUrl(): Promise<void> {
        const shareUrl = currentUrl();
        try {
            if (!canCopy()) throw new Error("clipboard unavailable");
            await navigator.clipboard.writeText(shareUrl);
            onCopy?.(shareUrl);
            if (copiedLabel) status = copiedLabel;
        } catch {
            if (copyFailedLabel) status = copyFailedLabel;
        }
        closeList();
    }
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
    class={`share-picker ${className}`.trim()}
    onfocusout={onRootFocusOut}
    {...restProps}
>
    <button
        bind:this={buttonEl}
        type="button"
        class="share-picker-button"
        aria-label={label}
        aria-expanded={open}
        aria-controls={listId}
        onclick={onButtonClick}
        onkeydown={onButtonKeydown}
    >
        {#if children}
            {@render children({ open, url: currentUrl() })}
        {:else}
            <svg
                class="share-picker-icon"
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
                <path d="M2.5 8h11M9 3.5 13.5 8 9 12.5" />
            </svg>
        {/if}
    </button>

    <!-- Named like the sibling pickers' listboxes: a screen reader
         entering the list hears what the list is for, not just "list,
         three items". -->
    <!-- The keydown handler is pure delegation for the focusable items
         inside (real links and a real button); the list itself is not
         interactive and takes no focus. -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <ul
        bind:this={listEl}
        class="share-picker-list"
        id={listId}
        aria-label={label}
        hidden={!open}
        onkeydown={onListKeydown}
    >
        {#each targets as target (target.id)}
            <li class="share-picker-list-item">
                <!-- A real link, not role="menuitem": these ARE navigation,
                     and menuitem would strip middle-click, open-in-new-tab
                     and copy-link-address. -->
                <a
                    class="share-picker-target"
                    data-target-id={target.id}
                    href={target.href(currentUrl(), title, text)}
                    target={target.newTab === false ? undefined : "_blank"}
                    rel="noopener noreferrer"
                    onclick={() => chooseTarget(target)}
                >
                    {target.label}
                </a>
            </li>
        {/each}

        {#if copyLabel}
            <li class="share-picker-list-item">
                <button type="button" class="share-picker-copy" onclick={copyUrl}>
                    {copyLabel}
                </button>
            </li>
        {/if}
    </ul>

    <!-- Copying gives no visual feedback of its own, so the outcome is
         announced. Empty until something happens, so it stays silent on
         load; aria-live announces mutations only. -->
    <p class="share-picker-status" aria-live="polite">{status}</p>
</div>
