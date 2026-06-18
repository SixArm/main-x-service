<script lang="ts" module>
    import type { StringKey } from '$lib/i18n.svelte';

    /**
     * One toolbar action: a stable `key` emitted on click, an i18n
     * `labelKey` resolved reactively to the visible label, and an `icon`
     * name resolved by the Icon component.
     */
    export interface BarButton {
        key: string;
        labelKey: StringKey;
        icon: string;
    }

    // The default action set from spec/button-bar.md. Labels are i18n keys
    // resolved at render time (so the bar relabels on a locale switch).
    export const DEFAULT_BUTTONS: BarButton[] = [
        { key: 'patient', labelKey: 'buttonBar.patient', icon: 'person' },
        { key: 'referrals', labelKey: 'buttonBar.referrals', icon: 'envelope-open' },
        { key: 'activate', labelKey: 'buttonBar.activate', icon: 'hospital' },
        { key: 'case-notes', labelKey: 'buttonBar.caseNotes', icon: 'folder' },
        { key: 'pathways', labelKey: 'buttonBar.pathways', icon: 'footprints' },
        { key: 'legal-status', labelKey: 'buttonBar.legalStatus', icon: 'scales' },
        { key: 'documents', labelKey: 'buttonBar.documents', icon: 'clipboard' },
        { key: 'wrapper', labelKey: 'buttonBar.wrapper', icon: 'box' },
        { key: 'audit', labelKey: 'buttonBar.audit', icon: 'magnifying-glass' },
        { key: 'quick-reports', labelKey: 'buttonBar.quickReports', icon: 'printer' }
    ];
</script>

<script lang="ts">
    // ButtonBar component
    //
    // A horizontal toolbar of labelled, icon-bearing buttons. Modelled on
    // the Lily Design System ButtonBar + Button. Headless — no CSS here.
    //
    // Props:
    //   buttons  — BarButton[], defaults to DEFAULT_BUTTONS (spec set).
    //   active   — string | undefined. Key of the currently-selected button.
    //   onselect — (key) => void. Called when a button is activated.
    //   label    — string, accessible name for the toolbar.
    //   class    — string, optional.

    import Button from '$lib/components/Button/Button.svelte';
    import Icon from '$lib/components/Icon/Icon.svelte';
    import { t } from '$lib/i18n.svelte';

    let {
        buttons = DEFAULT_BUTTONS,
        active = undefined,
        onselect = undefined,
        label = undefined,
        class: className = ''
    }: {
        buttons?: BarButton[];
        active?: string;
        onselect?: (key: string) => void;
        label?: string;
        class?: string;
    } = $props();

    // Default the accessible toolbar name to the translated "Actions".
    const toolbarLabel = $derived(label ?? t('buttonBar.label'));
</script>

<div class={`button-bar ${className}`} role="toolbar" aria-label={toolbarLabel}>
    {#each buttons as button (button.key)}
        <Button
            class="button-bar-button"
            type="button"
            pressed={active === undefined ? undefined : active === button.key}
            onclick={() => onselect?.(button.key)}
        >
            <Icon name={button.icon} />
            <span class="button-bar-label">{t(button.labelKey)}</span>
        </Button>
    {/each}
</div>
