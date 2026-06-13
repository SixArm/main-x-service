<script lang="ts" module>
    export interface BarButton {
        key: string;
        label: string;
        icon: string;
    }

    // The default action set from spec/button-bar.md.
    export const DEFAULT_BUTTONS: BarButton[] = [
        { key: 'patient', label: 'Patient', icon: 'person' },
        { key: 'referrals', label: 'Referrals', icon: 'envelope-open' },
        { key: 'activate', label: 'Activate', icon: 'hospital' },
        { key: 'case-notes', label: 'Case Notes', icon: 'folder' },
        { key: 'pathways', label: 'Pathways', icon: 'footprints' },
        { key: 'legal-status', label: 'Legal Status', icon: 'scales' },
        { key: 'documents', label: 'Documents', icon: 'clipboard' },
        { key: 'wrapper', label: 'Wrapper', icon: 'box' },
        { key: 'audit', label: 'Audit', icon: 'magnifying-glass' },
        { key: 'quick-reports', label: 'Quick Reports', icon: 'printer' }
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

    let {
        buttons = DEFAULT_BUTTONS,
        active = undefined,
        onselect = undefined,
        label = 'Actions',
        class: className = ''
    }: {
        buttons?: BarButton[];
        active?: string;
        onselect?: (key: string) => void;
        label?: string;
        class?: string;
    } = $props();
</script>

<div class={`button-bar ${className}`} role="toolbar" aria-label={label}>
    {#each buttons as button (button.key)}
        <Button
            class="button-bar-button"
            type="button"
            pressed={active === undefined ? undefined : active === button.key}
            onclick={() => onselect?.(button.key)}
        >
            <Icon name={button.icon} />
            <span class="button-bar-label">{button.label}</span>
        </Button>
    {/each}
</div>
