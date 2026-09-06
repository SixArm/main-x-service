<script lang="ts" module>
    /** One selectable row in the print-labels dialog: a volume's id + title. */
    export interface LabelOption {
        id: string;
        title: string;
    }
</script>

<script lang="ts">
    // LabelsDialogBox component (spec/labels-dialog-box.md)
    //
    // A dialog for printing labels for a selection of volumes:
    //   - Title "Labels"
    //   - search text + "Find" / "Clear"
    //   - a scrollable multi-select list of volume titles
    //   - footer: "Number of Copies:" stepper, "Print", "Close"
    //
    // Headless: structure + ARIA only.
    //
    // Props:
    //   open      — boolean, bindable. Whether the dialog is shown.
    //   volumes   — LabelOption[]. The selectable volume titles.
    //   copies    — number, bindable. Number of copies to print.
    //   selected  — string[], bindable. Selected volume ids.
    //   onprint   — ({ selected, copies }) => void.
    //   onclose   — () => void.

    import TextInput from '$lib/components/TextInput/TextInput.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import InputCount from '$lib/components/InputCount/InputCount.svelte';
    import { t } from '$lib/i18n.svelte';

    let {
        open = $bindable(true),
        volumes = [],
        copies = $bindable(1),
        selected = $bindable([]),
        onprint = undefined,
        onclose = undefined,
        class: className = '',
    }: {
        open?: boolean;
        volumes?: LabelOption[];
        copies?: number;
        selected?: string[];
        onprint?: (detail: { selected: string[]; copies: number }) => void;
        onclose?: () => void;
        class?: string;
    } = $props();

    // `search` is the live text box; `query` is the applied filter. They are
    // separate so filtering only happens on "Find" (not on every keystroke).
    let search = $state('');
    let query = $state('');

    // The visible options: case-insensitive title match against the applied
    // query, or the whole list when no query is set.
    const filtered = $derived(
        query.trim()
            ? volumes.filter((v) =>
                  v.title.toLowerCase().includes(query.trim().toLowerCase()),
              )
            : volumes,
    );

    // Apply the current search text as the active filter.
    function find() {
        query = search;
    }
    // Reset search text, the applied filter, and the selection.
    function clear() {
        search = '';
        query = '';
        selected = [];
    }
    function print() {
        onprint?.({ selected, copies });
    }
    function close() {
        open = false;
        onclose?.();
    }
</script>

{#if open}
    <div class="dialog-backdrop">
        <div
            class={`labels-dialog ${className}`}
            role="dialog"
            aria-modal="true"
            aria-labelledby="labels-dialog-title"
        >
            <h2 id="labels-dialog-title">{t('labels.title')}</h2>

            <div class="labels-dialog-search">
                <TextInput
                    label={t('labels.searchLabel')}
                    placeholder={t('labels.searchPlaceholder')}
                    bind:value={search}
                />
                <Button type="button" onclick={find}>{t('labels.find')}</Button>
                <Button type="button" onclick={clear}
                    >{t('labels.clear')}</Button
                >
            </div>

            <ul
                class="labels-dialog-list"
                role="listbox"
                aria-multiselectable="true"
                aria-label={t('labels.volumeTitles')}
            >
                {#each filtered as option (option.id)}
                    <li
                        role="option"
                        aria-selected={selected.includes(option.id)}
                    >
                        <label>
                            <input
                                type="checkbox"
                                value={option.id}
                                bind:group={selected}
                            />
                            {option.title}
                        </label>
                    </li>
                {/each}
                {#if filtered.length === 0}
                    <li class="labels-dialog-empty">
                        {t('labels.noMatching')}
                    </li>
                {/if}
            </ul>

            <footer class="labels-dialog-footer">
                <label class="labels-dialog-copies">
                    {t('labels.numberOfCopies')}
                    <InputCount
                        label={t('labels.copiesLabel')}
                        bind:value={copies}
                    />
                </label>
                <div class="labels-dialog-actions">
                    <Button type="button" onclick={print}
                        >{t('labels.print')}</Button
                    >
                    <Button type="button" onclick={close}
                        >{t('labels.close')}</Button
                    >
                </div>
            </footer>
        </div>
    </div>
{/if}
