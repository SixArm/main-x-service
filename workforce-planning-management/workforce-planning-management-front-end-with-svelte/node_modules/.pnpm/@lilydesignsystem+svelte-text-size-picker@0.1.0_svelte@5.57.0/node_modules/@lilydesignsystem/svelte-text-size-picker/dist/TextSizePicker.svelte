<script lang="ts" module>
    import type { Snippet } from "svelte";

    /**
     * Default button icon: a bundled SVG (a stroke-drawn "A"), not a
     * Unicode character. Reversed 2026-09-16 from the font-dependent-glyph
     * convention (was the plain letter U+0041, exported as
     * `LATIN_CAPITAL_LETTER_A` — removed, not renamed). "A" itself needed
     * no escaping and had no font-fallback risk, but it still varied in
     * weight and proportions across font stacks; a bundled outline SVG
     * matches the other four picker icons as one consistent visual family
     * regardless of the consumer's fonts.
     */

    /** Arguments passed to a custom `children` snippet (the button glyph). */
    export type ChildArgs = {
        /** Currently selected size slug. */
        value: string;
        /** Is the listbox open? */
        open: boolean;
        /** Resolve a slug to its display label. */
        labelFor: (size: string) => string;
    };

    /** Public props for TextSizePicker. See `spec/index.md` §4 for the contract. */
    export type Props = {
        /** Accessible name for the button and the listbox. */
        label: string;
        /** Available size slugs, e.g. ["small","medium","large","x-large"]. */
        sizes: string[];
        /** Currently selected size slug. Two-way bindable. */
        value?: string;
        /** Initial size when nothing else is supplied. */
        defaultValue?: string;
        /** If set, persist the selection to localStorage under this key. */
        storageKey?: string;
        /** `name` of the hidden input that carries the value in a form. */
        name?: string;
        /** Element that receives `data-text-size`. Defaults to document.documentElement. */
        target?: HTMLElement | null;
        /** Optional pretty labels per slug. */
        sizeLabels?: Record<string, string>;
        /** Replaces the default "A" icon inside the button. */
        children?: Snippet<[ChildArgs]>;
        /** Called after the control applies a new size. */
        onChange?: (size: string) => void;
        /** Extra CSS class on the root. */
        class?: string;
        /** Spread props onto the root element. */
        [key: string]: unknown;
    };

    /**
     * Resolve a size slug to its display label: each hyphen-separated word
     * title-cased, so "x-large" renders as "X Large". Mirrors `themeName`
     * in theme-picker and `localeName` in locale-picker.
     */
    export function sizeName(size: string): string {
        return size
            .split("-")
            .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
            .join(" ");
    }

    let uid = 0;
    /** Stable per-instance id prefix; SSR-safe (no Math.random / Date.now). */
    export function nextTextSizePickerId(): string {
        uid += 1;
        return `text-size-picker-${uid}`;
    }
</script>

<script lang="ts">
    let {
        class: className = "",
        label,
        sizes,
        value = $bindable(""),
        defaultValue,
        storageKey,
        name = "text-size",
        target,
        sizeLabels = {},
        children,
        onChange,
        ...restProps
    }: Props = $props();

    const baseId = nextTextSizePickerId();
    const listId = `${baseId}-list`;
    const optionId = (i: number) => `${baseId}-option-${i}`;

    let open = $state(false);
    let activeIndex = $state(-1);
    let buttonEl: HTMLButtonElement | undefined = $state();
    let listEl: HTMLUListElement | undefined = $state();
    let rootEl: HTMLDivElement | undefined = $state();

    // Typeahead buffer: APG listbox behaviour. Reset after a pause.
    let typeahead = "";
    let typeaheadTimer: ReturnType<typeof setTimeout> | undefined;

    function labelFor(size: string): string {
        if (size in sizeLabels) return sizeLabels[size];
        return sizeName(size);
    }

    // The size the DOM currently carries. Applying is idempotent: the
    // effect below can run for reasons other than a size change, and
    // re-applying would re-fire `onChange`. A consumer whose onChange
    // writes reactive state then re-enters this effect, and Svelte stops
    // updating the component altogether (effect_update_depth_exceeded) —
    // the listbox freezes mid-open with a stale aria-expanded. Guarding
    // here also matches the spec: other prop changes are not retroactive.
    let appliedValue = "";

    function applySize(slug: string): void {
        if (typeof document === "undefined" || !slug) return;
        if (slug === appliedValue) return;
        appliedValue = slug;
        (target ?? document.documentElement).setAttribute("data-text-size", slug);
        if (storageKey) {
            try {
                localStorage.setItem(storageKey, slug);
            } catch {
                // ignore quota / privacy errors
            }
        }
        onChange?.(slug);
    }

    function setSize(slug: string): void {
        value = slug;
    }

    // ---------------------------------------------------------------
    // Open / close
    // ---------------------------------------------------------------

    function openList(startIndex?: number): void {
        const selected = sizes.indexOf(value);
        // An empty list has no option to activate; -1 keeps
        // aria-activedescendant off rather than pointing at an id that
        // does not exist.
        activeIndex =
            sizes.length === 0
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
        const slug = sizes[index];
        if (slug) setSize(slug);
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
        el?.scrollIntoView?.({ block: "nearest" });
    }

    function moveActive(delta: number): void {
        if (sizes.length === 0) return;
        const next = Math.min(Math.max(activeIndex + delta, 0), sizes.length - 1);
        activeIndex = next;
        scrollActiveIntoView();
    }

    function runTypeahead(char: string): void {
        const lower = char.toLowerCase();
        // APG listbox typeahead: a single character moves to the NEXT
        // option starting with it, and repeating that character keeps
        // cycling. Only a buffer of differing characters refines the
        // match, and that buffer stays anchored on the active option.
        const sameCharRun =
            typeahead === "" || [...typeahead].every((c) => c === lower);
        typeahead += lower;
        clearTimeout(typeaheadTimer);
        typeaheadTimer = setTimeout(() => (typeahead = ""), 500);
        const query = sameCharRun ? lower : typeahead;
        const anchor = activeIndex < 0 ? 0 : activeIndex;
        const start = sameCharRun ? anchor + 1 : anchor;
        // Search forward, wrapping once — typeahead wraps even though the
        // arrows clamp, or options above the cursor would be untypable.
        for (let n = 0; n < sizes.length; n++) {
            const i = (start + n) % sizes.length;
            if (labelFor(sizes[i]).toLowerCase().startsWith(query)) {
                activeIndex = i;
                scrollActiveIntoView();
                return;
            }
        }
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
                openList(sizes.length - 1);
                break;
        }
    }

    function onListKeydown(event: KeyboardEvent): void {
        switch (event.key) {
            case "ArrowDown":
                event.preventDefault();
                moveActive(1);
                break;
            case "ArrowUp":
                event.preventDefault();
                moveActive(-1);
                break;
            case "Home":
                event.preventDefault();
                activeIndex = 0;
                scrollActiveIntoView();
                break;
            case "End":
                event.preventDefault();
                activeIndex = sizes.length - 1;
                scrollActiveIntoView();
                break;
            case "Enter":
            case " ":
                event.preventDefault();
                if (activeIndex >= 0) choose(activeIndex);
                break;
            case "Escape":
                event.preventDefault();
                closeList();
                break;
            case "PageUp":
                event.preventDefault();
                moveActive(-10);
                break;
            case "PageDown":
                // ±10, clamped: an APG-optional key, matching the
                // sibling pickers.
                event.preventDefault();
                moveActive(10);
                break;
            case "Tab":
                // Tab moves on — but focus goes to the button FIRST,
                // without cancelling the key. Hiding the focused list
                // drops focus to <body>, and the browser then computes
                // the default Tab move from the top of the document, so
                // tabbing out of an open picker teleported the user to
                // the page's first tab stop. From the button, the default
                // Tab lands exactly where leaving the picker should.
                buttonEl?.focus?.({ preventScroll: true });
                closeList(false);
                break;
            default:
                if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
                    runTypeahead(event.key);
                }
        }
    }

    function onRootFocusOut(event: FocusEvent): void {
        const next = event.relatedTarget as Node | null;
        if (next && rootEl?.contains(next)) return;
        closeList(false);
    }

    // ---------------------------------------------------------------
    // Initial value resolution + apply
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
            if (!initial) {
                initial =
                    defaultValue ??
                    (sizes.includes("medium") ? "medium" : sizes[0]) ??
                    "";
            }
            if (initial && initial !== current) {
                value = initial;
                return;
            }
        }

        if (current) applySize(current);
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
    class={`text-size-picker ${className}`.trim()}
    onfocusout={onRootFocusOut}
    {...restProps}
>
    <input type="hidden" {name} {value} />

    <button
        bind:this={buttonEl}
        type="button"
        class="text-size-picker-button"
        aria-label={label}
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
                class="text-size-picker-icon"
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
                <path d="M4 13 7.2 3h1.6L12 13M5.4 9.5h5.2" />
            </svg>
        {/if}
    </button>

    <ul
        bind:this={listEl}
        class="text-size-picker-list"
        id={listId}
        role="listbox"
        aria-label={label}
        aria-activedescendant={open && activeIndex >= 0 ? optionId(activeIndex) : undefined}
        tabindex="-1"
        hidden={!open}
        onkeydown={onListKeydown}
    >
        {#each sizes as size, i (size)}
            <!-- The option's keyboard interaction lives on the listbox
                 (aria-activedescendant pattern): the list is the focused
                 element and its keydown handler operates the options, so a
                 per-option key handler would be wrong, not missing. -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <li
                class="text-size-picker-option"
                id={optionId(i)}
                role="option"
                aria-selected={size === value}
                data-active={i === activeIndex ? "" : undefined}
                onclick={() => choose(i)}
            >
                {labelFor(size)}
            </li>
        {/each}
    </ul>
</div>
