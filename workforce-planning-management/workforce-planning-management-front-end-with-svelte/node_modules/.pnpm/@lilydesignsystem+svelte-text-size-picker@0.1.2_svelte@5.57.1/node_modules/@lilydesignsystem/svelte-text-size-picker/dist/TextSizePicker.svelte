<script lang="ts" module>
    import type { Snippet } from "svelte";
    import { IconButton, Listbox } from "@lilydesignsystem/svelte-headless";

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
    let listEl: HTMLElement | undefined = $state();
    let rootEl: HTMLDivElement | undefined = $state();

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
                openList(sizes.length - 1);
                break;
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

    <IconButton
        bind:ref={buttonEl}
        baseClass="text-size-picker-button"
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
    </IconButton>

    <Listbox
        bind:ref={listEl}
        as="ul"
        baseClass="text-size-picker-list"
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
    </Listbox>
</div>
