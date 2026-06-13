<script lang="ts" module>
    import type { IconDefinition } from '@fortawesome/fontawesome-svg-core';
    import {
        faUser,
        faEnvelopeOpen,
        faHospital,
        faFolder,
        faShoePrints,
        faScaleBalanced,
        faClipboard,
        faBoxArchive,
        faMagnifyingGlass,
        faPrint,
        faXmark,
        faPlus,
        faMinus
    } from '@fortawesome/free-solid-svg-icons';

    // Named FontAwesome (free, solid) icons used across the app. Rendered
    // as inline SVG — no CDN, no kit script — so it satisfies the project's
    // CSP / offline posture (see spec/regulatory.md).
    export const ICONS: Record<string, IconDefinition> = {
        person: faUser,
        'envelope-open': faEnvelopeOpen,
        hospital: faHospital,
        folder: faFolder,
        footprints: faShoePrints,
        scales: faScaleBalanced,
        clipboard: faClipboard,
        box: faBoxArchive,
        'magnifying-glass': faMagnifyingGlass,
        printer: faPrint,
        xmark: faXmark,
        plus: faPlus,
        minus: faMinus
    };

    export type IconName = keyof typeof ICONS;
</script>

<script lang="ts">
    // Icon component
    //
    // Renders a single FontAwesome (free, solid) glyph as inline SVG.
    //
    // Props:
    //   name  — string, required. One of the keys in ICONS.
    //   label — string, optional. Accessible label. When omitted the icon
    //           is decorative (aria-hidden) — use this inside a button that
    //           already has visible text.
    //   class — string, optional.
    //
    // Claude rules:
    //   - Headless: no CSS. Sized via `1em`, coloured via `currentColor`.

    let {
        name,
        label = undefined,
        class: className = ''
    }: { name: string; label?: string; class?: string } = $props();

    const def = $derived(ICONS[name]);
</script>

{#if def}
    {@const width = def.icon[0]}
    {@const height = def.icon[1]}
    {@const path = def.icon[4]}
    <svg
        class={`icon ${className}`}
        viewBox={`0 0 ${width} ${height}`}
        width="1em"
        height="1em"
        fill="currentColor"
        role={label ? 'img' : 'presentation'}
        aria-label={label}
        aria-hidden={label ? undefined : 'true'}
        focusable="false"
    >
        <path d={Array.isArray(path) ? path.join(' ') : path} />
    </svg>
{/if}
